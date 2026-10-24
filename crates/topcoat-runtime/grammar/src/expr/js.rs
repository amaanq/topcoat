use std::{fmt, ops::AddAssign};

use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use topcoat_core_grammar::paths::topcoat_runtime;

/// JavaScript source with holes for expressions captured by the server.
///
/// Expands to a registered template applied to the captured expressions'
/// JavaScript, so the source ships in the template module and the page only
/// carries the captured values.
#[derive(Default)]
pub(super) struct Js {
    source: String,
    holes: Vec<Ident>,
}

impl Js {
    pub(super) fn push_str(&mut self, source: &str) {
        self.source.push_str(source);
    }

    pub(super) fn push(&mut self, character: char) {
        self.source.push(character);
    }

    pub(super) fn expression(&mut self, ident: &Ident) {
        let index = self
            .holes
            .iter()
            .position(|hole| hole == ident)
            .unwrap_or_else(|| {
                self.holes.push(ident.clone());
                self.holes.len() - 1
            });
        self.source.push_str("(__holes[");
        self.source.push_str(&index.to_string());
        self.source.push_str("]())");
    }
}

impl ToTokens for Js {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let source = format!("(cx, __holes) => ({})", self.source);
        let holes = &self.holes;
        // Bound to a local because it is interpolated inside the `#(...)*`
        // repetition below, where a bare `#topcoat_runtime` would expand to
        // a `let` binding that cannot shadow the imported constant.
        let js = quote!(#topcoat_runtime::Expr::js);
        quote! {
            #topcoat_runtime::js_template!(
                #source
                #(, ::core::clone::Clone::clone(#js(&#holes)))*
            )
        }
        .to_tokens(tokens);
    }
}

impl fmt::Write for Js {
    fn write_str(&mut self, source: &str) -> fmt::Result {
        self.push_str(source);
        Ok(())
    }
}

impl AddAssign<&str> for Js {
    fn add_assign(&mut self, source: &str) {
        self.push_str(source);
    }
}
