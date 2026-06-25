use darling::{ast, FromDeriveInput, FromField};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Type};

// ── Field-level attributes ────────────────────────────────────────────────────

#[derive(Debug, FromField)]
#[darling(attributes(field))]
struct FieldOpts {
    ident: Option<syn::Ident>,
    ty: Type,
    /// Mark field as required (non-nullable)
    #[darling(default)]
    required: bool,
    /// Default value expression as string, e.g. default = "0"
    #[darling(default)]
    default: Option<String>,
    /// Minimum length for String fields
    #[darling(default)]
    min_length: Option<usize>,
    /// Maximum length for String fields
    #[darling(default)]
    max_length: Option<usize>,
    /// Enum of allowed values (comma-separated string)
    #[darling(default)]
    enum_values: Option<String>,
    /// Mark as unique (informational — enforced via index on MongoDB)
    #[darling(default)]
    unique: bool,
    /// Rename the field in MongoDB
    #[darling(default)]
    rename: Option<String>,
}

// ── Schema-level attributes ───────────────────────────────────────────────────

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(schema), supports(struct_named))]
struct SchemaOpts {
    ident: syn::Ident,
    data: ast::Data<(), FieldOpts>,
    /// Override MongoDB collection name
    #[darling(default)]
    collection: Option<String>,
    /// Automatically add created_at / updated_at
    #[darling(default)]
    timestamps: bool,
}

// ── #[derive(Schema)] ─────────────────────────────────────────────────────────

#[proc_macro_derive(Schema, attributes(schema, field))]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let opts = match SchemaOpts::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    let name = &opts.ident;
    let name_str = name.to_string();

    let collection_name = opts
        .collection
        .clone()
        .unwrap_or_else(|| to_snake_case_plural(&name_str));

    let timestamps = opts.timestamps;

    let fields = opts.data.take_struct().expect("only named structs");

    // Build per-field validation logic
    let validations: Vec<TokenStream2> = fields
        .fields
        .iter()
        .filter_map(|f| build_field_validation(f))
        .collect();

    // Build list of unique field names for index hints
    let unique_fields: Vec<String> = fields
        .fields
        .iter()
        .filter(|f| f.unique)
        .filter_map(|f| {
            f.rename.clone().or_else(|| {
                f.ident
                    .as_ref()
                    .map(|i| i.to_string())
            })
        })
        .collect();

    let unique_field_literals: Vec<proc_macro2::Literal> = unique_fields
        .iter()
        .map(|s| proc_macro2::Literal::string(s))
        .collect();

    let timestamps_impl = if timestamps {
        quote! {
            fn timestamps() -> bool { true }
        }
    } else {
        quote! {
            fn timestamps() -> bool { false }
        }
    };

    let expanded = quote! {
        impl mongrel::schema::MongooseSchema for #name {
            fn collection_name() -> &'static str {
                #collection_name
            }

            #timestamps_impl

            fn unique_fields() -> &'static [&'static str] {
                &[#(#unique_field_literals),*]
            }

            fn validate(&self) -> std::result::Result<(), mongrel::error::MongooseError> {
                #(#validations)*
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}

// ── #[derive(Model)] — gives a type its static Model handle ──────────────────

#[proc_macro_derive(Model, attributes(schema, field))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let model_name = quote::format_ident!("{}Model", name);

    let expanded = quote! {
        pub struct #model_name;

        impl #model_name {
            pub fn new(db: std::sync::Arc<mongodb::Database>) -> mongrel::model::Model<#name> {
                mongrel::model::Model::new(db)
            }
        }
    };

    TokenStream::from(expanded)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn build_field_validation(f: &FieldOpts) -> Option<TokenStream2> {
    let ident = f.ident.as_ref()?;
    let field_name = ident.to_string();
    let mut checks = Vec::new();

    if let Some(min) = f.min_length {
        checks.push(quote! {
            if let Some(s) = mongrel::schema::AsStr::as_str_opt(&self.#ident) {
                if s.len() < #min {
                    return Err(mongrel::error::MongooseError::Validation(
                        format!("Field `{}` must be at least {} characters", #field_name, #min)
                    ));
                }
            }
        });
    }

    if let Some(max) = f.max_length {
        checks.push(quote! {
            if let Some(s) = mongrel::schema::AsStr::as_str_opt(&self.#ident) {
                if s.len() > #max {
                    return Err(mongrel::error::MongooseError::Validation(
                        format!("Field `{}` must be at most {} characters", #field_name, #max)
                    ));
                }
            }
        });
    }

    if let Some(enum_str) = &f.enum_values {
        let allowed: Vec<&str> = enum_str.split(',').map(str::trim).collect();
        let allowed_literals: Vec<proc_macro2::Literal> = allowed
            .iter()
            .map(|s| proc_macro2::Literal::string(s))
            .collect();
        checks.push(quote! {
            if let Some(s) = mongrel::schema::AsStr::as_str_opt(&self.#ident) {
                let allowed = &[#(#allowed_literals),*];
                if !allowed.contains(&s) {
                    return Err(mongrel::error::MongooseError::Validation(
                        format!("Field `{}` must be one of {:?}", #field_name, allowed)
                    ));
                }
            }
        });
    }

    if checks.is_empty() {
        None
    } else {
        Some(quote! { #(#checks)* })
    }
}

fn to_snake_case_plural(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            out.push('_');
        }
        out.push(c.to_lowercase().next().unwrap());
    }
    out.push('s');
    out
}
