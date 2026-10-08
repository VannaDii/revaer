use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::LitStr;

pub(crate) struct Doc {
    pub hidden: bool,
    fragments: Vec<LitStr>,
}

impl Doc {
    pub(crate) fn new() -> Self {
        Doc {
            hidden: false,
            fragments: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, lit: LitStr) {
        self.fragments.push(lit);
    }

    #[cfg_attr(proc_macro, expect(dead_code))]
    pub(crate) fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    #[cfg_attr(proc_macro, expect(dead_code))]
    pub(crate) fn to_string(&self) -> String {
        let mut doc = String::new();
        for lit in &self.fragments {
            doc += &lit.value();
            doc.push('\n');
        }
        doc
    }
}

impl ToTokens for Doc {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let fragments = &self.fragments;
        for fragment in fragments {
            tokens.extend(quote_spanned! {fragment.span()=> #[doc = #fragment]});
        }
        if self.hidden {
            tokens.extend(quote! { #[doc(hidden)] });
        }
    }
}
