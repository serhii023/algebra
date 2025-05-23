use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, Index, Type};

pub(crate) enum IdentOrIndex {
    Ident(proc_macro2::Ident),
    Index(Index),
}

impl ToTokens for IdentOrIndex {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::Index(index) => index.to_tokens(tokens),
        }
    }
}

fn impl_serialize_field(
    serialize_body: &mut Vec<TokenStream>,
    serialized_size_body: &mut Vec<TokenStream>,
    idents: &mut Vec<IdentOrIndex>,
    ty: &Type,
) {
    // Check if type is a tuple.
    match ty {
        Type::Tuple(tuple) => {
            for (i, elem_ty) in tuple.elems.iter().enumerate() {
                let index = Index::from(i);
                idents.push(IdentOrIndex::Index(index));
                impl_serialize_field(serialize_body, serialized_size_body, idents, elem_ty);
                idents.pop();
            }
        },
        _ => {
            serialize_body
                .push(quote! { CanonicalSerialize::serialize_with_mode(&self.#(#idents).*, &mut writer, compress)?; });
            serialized_size_body
                .push(quote! { size += CanonicalSerialize::serialized_size(&self.#(#idents).*, compress); });
        },
    }
}

pub(super) fn impl_canonical_serialize(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let len = if let Data::Struct(ref data_struct) = ast.data {
        data_struct.fields.len()
    } else {
        panic!(
            "`CanonicalSerialize` can only be derived for structs, {} is not a struct",
            name
        );
    };

    let mut serialize_body = Vec::<TokenStream>::with_capacity(len);
    let mut serialized_size_body = Vec::<TokenStream>::with_capacity(len);

    match ast.data {
        Data::Struct(ref data_struct) => {
            let mut idents = Vec::<IdentOrIndex>::new();

            for (i, field) in data_struct.fields.iter().enumerate() {
                match field.ident {
                    None => {
                        let index = Index::from(i);
                        idents.push(IdentOrIndex::Index(index));
                    },
                    Some(ref ident) => {
                        idents.push(IdentOrIndex::Ident(ident.clone()));
                    },
                }

                impl_serialize_field(
                    &mut serialize_body,
                    &mut serialized_size_body,
                    &mut idents,
                    &field.ty,
                );

                idents.clear();
            }
        },
        _ => panic!(
            "`CanonicalSerialize` can only be derived for structs, {} is not a struct",
            name
        ),
    };

    let gen = quote! {
        impl #impl_generics ark_serialize::CanonicalSerialize for #name #ty_generics #where_clause {
            fn serialize_with_mode<W: ark_serialize::Write>(&self, mut writer: W, compress: ark_serialize::Compress) -> Result<(), ark_serialize::SerializationError> {
                #(#serialize_body)*
                Ok(())
            }
            fn serialized_size(&self, compress: ark_serialize::Compress) -> usize {
                let mut size = 0;
                #(#serialized_size_body)*
                size
            }
        }
    };
    gen
}
