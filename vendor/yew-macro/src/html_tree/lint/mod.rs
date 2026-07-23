//! Lints to catch possible misuse of the `html!` macro use. At the moment these are mostly focused
//! on accessibility.

use proc_macro_error3::emit_warning;
use syn::spanned::Spanned;

use super::html_element::{HtmlElement, TagName};
use super::HtmlTree;
use crate::props::{ElementProps, Prop};

/// Lints HTML elements to check if they are well formed. If the element is not well-formed, then
/// use `proc-macro-error` (and the `emit_warning!` macro) to produce a warning. At present, these
/// are only emitted on nightly.
pub trait Lint {
    #[cfg_attr(not(yew_lints), allow(dead_code))]
    fn lint(element: &HtmlElement);
}

/// Applies all the lints to the HTML tree.
pub fn lint_all(tree: &HtmlTree) {
    lint::<AHrefLint>(tree);
    lint::<ImgAltLint>(tree);
}

/// Applies a specific lint to the HTML tree.
pub fn lint<L>(tree: &HtmlTree)
where
    L: Lint,
{
    let _ = L::lint;
    #[cfg(not(yew_lints))]
    let _ = tree;
    #[cfg(yew_lints)]
    match tree {
        HtmlTree::List(list) => {
            for child in &list.children.0 {
                lint::<L>(child)
            }
        }
        HtmlTree::Element(el) => L::lint(el),
        _ => {}
    }
}

/// Retrieves an attribute from an element and returns a reference valid for the lifetime of the
/// element (if that attribute can be found on the prop).
///
/// Attribute names are lowercased before being compared (so pass "href" for `name` and not "HREF").
fn get_attribute<'a>(props: &'a ElementProps, name: &str) -> Option<&'a Prop> {
    props
        .attributes
        .iter()
        .find(|item| item.label.eq_ignore_ascii_case(name))
}

fn invalid_href(prop: &Prop) -> Option<&syn::LitStr> {
    let syn::Expr::Lit(expression) = &prop.value else {
        return None;
    };
    let syn::Lit::Str(href) = &expression.lit else {
        return None;
    };
    matches!(href.value().as_str(), "#" | "javascript:void(0)").then_some(href)
}

/// Lints to check if anchor (`<a>`) tags have valid `href` attributes defined.
pub struct AHrefLint;

impl Lint for AHrefLint {
    fn lint(element: &HtmlElement) {
        let TagName::Lit(tag_name) = &element.name else {
            return;
        };
        if !tag_name.eq_ignore_ascii_case("a") {
            return;
        }
        let Some(prop) = get_attribute(&element.props, "href") else {
            emit_warning!(
                quote::quote! {#tag_name}.span(),
                "All `<a>` elements should have a `href` attribute. This makes it possible \
                    for assistive technologies to correctly interpret what your links point to. \
                    https://developer.mozilla.org/en-US/docs/Learn/Accessibility/HTML#more_on_links"
            );
            return;
        };
        if let Some(href) = invalid_href(prop) {
            let href_value = href.value();
            emit_warning!(
                href.span(),
                format!("'{href_value}' is not a suitable value for the `href` attribute. \
                        Without a meaningful attribute assistive technologies \
                        will struggle to understand your webpage. \
                        https://developer.mozilla.org/en-US/docs/Learn/Accessibility/HTML#onclick_events"
            ));
        }
    }
}

/// Checks to make sure that images have `alt` attributes defined.
pub struct ImgAltLint;

impl Lint for ImgAltLint {
    fn lint(element: &HtmlElement) {
        if let super::html_element::TagName::Lit(ref tag_name) = element.name {
            if !tag_name.eq_ignore_ascii_case("img") {
                return;
            };
            if get_attribute(&element.props, "alt").is_none() {
                emit_warning!(
                    quote::quote! {#tag_name}.span(),
                    "All `<img>` tags should have an `alt` attribute which provides a \
                     human-readable description "
                )
            }
        }
    }
}
