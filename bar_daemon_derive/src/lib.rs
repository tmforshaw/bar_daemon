use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

// Derive Changed
#[proc_macro_derive(Changed)]
pub fn changed_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the struct name, and add Changed to the end
    let type_name = input.ident;
    let changed_name = syn::Ident::new(&format!("{type_name}Changed"), type_name.span());

    // Extract generics and type parameters
    let generics = input.generics.clone();
    let type_params: Vec<_> = generics.type_params().map(|tp| &tp.ident).collect();

    // Get all the struct's field names
    let fields = match input.data {
        Data::Struct(ref data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("Changed only supports named fields"),
        },
        _ => panic!("Changed can only be derived for structs"),
    };

    // Build the {Name}Changed struct fields
    let changed_fields = fields.iter().map(|f| {
        let fname = &f.ident;
        quote! { #fname: bool }
    });

    // Mark which fields have changed
    let comparisons = fields.iter().map(|f| {
        let fname = &f.ident;
        quote! { #fname: self.#fname != other.#fname }
    });

    // Generate all_true() and all_false() fields -------------------

    // Generate all_true() initializer with all fields true
    let all_true_fields = fields.iter().map(|f| {
        let fname = &f.ident;
        quote! { #fname: true }
    });

    // Generate all_false() initializer with all fields false
    let all_false_fields = fields.iter().map(|f| {
        let fname = &f.ident;
        quote! { #fname: false }
    });

    // Generate Documentation -------------------

    // Generate docs for changed()
    let changed_struct_docs = {
        let text = format!("# Documentation\nStruct of `{changed_name}` to allow implementing `Changed` for `{type_name}`");
        LitStr::new(&text, proc_macro2::Span::call_site())
    };

    // Generate docs for changed()
    let changed_fn_docs = {
        let text =
            format!("# Documentation\nGet the `bool` for each field of `{type_name}` which changed between `self` and `other`");
        LitStr::new(&text, proc_macro2::Span::call_site())
    };

    // Generate docs for ChangedConstructor all_true()
    let all_true_fn_docs = {
        let text = format!("# Documentation\nCreates a new `{changed_name}` with all fields initialised to `true`");
        LitStr::new(&text, proc_macro2::Span::call_site())
    };

    // Generate docs for ChangedConstructor all_true()
    let all_false_fn_docs = {
        let text = format!("# Documentation\nCreates a new `{changed_name}` with all fields initialised to `false`");
        LitStr::new(&text, proc_macro2::Span::call_site())
    };

    // Generate the Struct, and Impls for the Changed trait and ChangedConstructor trait
    let expanded = quote! {
        // Create {Name}Changed struct
        #[doc = #changed_struct_docs]
        pub struct #changed_name #generics {
           #( #changed_fields ),*
        }

        // Implement Changed trait
        impl #generics Changed for #type_name #generics {
            type ChangedType = #changed_name #generics;

            #[doc = #changed_fn_docs]
            fn changed(&self, other: &Self) -> Self::ChangedType
            where #( #type_params: std::cmp::PartialEq ),*
            {
                Self::ChangedType {
                   #( #comparisons ),*
                }
            }
        }

         // Implement ChangedConstructor for the ChangedType struct
         impl #generics ChangedConstructor for #changed_name #generics {
            #[doc = #all_true_fn_docs]
            fn all_true() -> Self {
                Self {
                    #( #all_true_fields ),*
                }
            }

            #[doc = #all_false_fn_docs]
            fn all_false() -> Self {
                Self {
                    #( #all_false_fields ),*
                }
            }
         }
    };

    TokenStream::from(expanded)
}

// Derive IntoSnapshotEvent
#[proc_macro_derive(IntoSnapshotEvent)]
pub fn derive_into_snapshot_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = input.ident;

    let into_event_docs = {
        let text = format!("# Documentation\nCreates a `SnapshotEvent` from a value of `MonitoredUpdate<{type_name}>`");
        LitStr::new(&text, proc_macro2::Span::call_site())
    };

    let from_update_docs = {
        let text = format!("# Documentation\nCreates a `SnapshotEvent` from a value of `MonitoredUpdate<{type_name}>`");
        LitStr::new(&text, proc_macro2::Span::call_site())
    };

    let expanded = quote! {
        impl IntoSnapshotEvent for #type_name {
            #[doc = #into_event_docs]
            fn into_event(update: MonitoredUpdate<Self>) -> SnapshotEvent {
                SnapshotEvent::#type_name(update)
            }
        }

        impl From<MonitoredUpdate<#type_name>> for SnapshotEvent {
            #[doc = #from_update_docs]
            fn from(update: MonitoredUpdate<#type_name>) -> Self {
                <#type_name as IntoSnapshotEvent>::into_event(update)
            }
        }
    };

    TokenStream::from(expanded)
}

// Derive Polled
#[proc_macro_derive(Polled)]
pub fn derive_polled(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = input.ident;

    let docs = {
        let text = format!(
            "# Documentation\nGets the latest value of `{type_name}`\n\n# Errors\nReturns an `Err` if `{type_name}::latest()` fails"
        );
        LitStr::new(&text, proc_macro2::Span::call_site())
    };

    let expanded = quote! {
        impl Polled for #type_name {
            #[doc = #docs]
            async fn poll() -> Result<Observed<Self>, DaemonError> {
                Self::latest().await
            }
        }
    };

    TokenStream::from(expanded)
}
