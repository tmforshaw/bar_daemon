use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Changed)]
pub fn changed_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the struct name, and add Changed to the end
    let name = input.ident;
    let changed_name = syn::Ident::new(&format!("{name}Changed"), name.span());

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

    // Build the Changed struct fields
    let changed_fields = fields.iter().map(|f| {
        let fname = &f.ident;
        quote! { #fname: bool }
    });

    // Build the changed() method body
    let comparisons = fields.iter().map(|f| {
        let fname = &f.ident;
        quote! { #fname: self.#fname != other.#fname }
    });

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

    let expanded = quote! {
        /// # Documentation
        /// Generate Changed struct
        pub struct #changed_name #generics {
           #( #changed_fields ),*
        }

        // Implement Changed trait
        impl #generics Changed for #name #generics {
            type ChangedType = #changed_name #generics;

            /// # Documentation
            /// Creates a `changed()` function which returns which fields changed between `self` and `other`
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
            /// # Documentation
            /// Creates a new `ChangedType` with all fields initialised to `true`
            fn all_true() -> Self {
                Self {
                    #( #all_true_fields ),*
                }
            }

            /// # Documentation
            /// Creates a new `ChangedType` with all fields initialised to `false`
            fn all_false() -> Self {
                Self {
                    #( #all_false_fields ),*
                }
            }
         }
    };

    TokenStream::from(expanded)
}
