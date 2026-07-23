use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Token, Type};

use super::{HtmlChildrenTree, TagTokens};
use crate::is_ide_completion;
use crate::props::ComponentProps;

fn is_component_close(input: ParseStream) -> bool {
    input.peek(Token![<]) && input.peek2(Token![/])
}

fn format_token_stream(tokens: impl ToTokens) -> String {
    tokens.to_token_stream().to_string().replace(' ', "")
}

fn reject_orphan_component_close(input: ParseStream) -> syn::Result<()> {
    if !is_component_close(input) {
        return Ok(());
    }
    let close = input.parse::<HtmlComponentClose>();
    if is_ide_completion() {
        return Ok(());
    }
    match close {
        Ok(close) => Err(syn::Error::new_spanned(
            close.to_spanned(),
            "this closing tag has no corresponding opening tag",
        )),
        Err(error) => Err(error),
    }
}

fn parse_component_close(
    input: ParseStream,
    open: &HtmlComponentOpen,
) -> syn::Result<HtmlComponentClose> {
    let input_fork = input.fork();
    TagTokens::parse_end_content(&input_fork, |content, tag| {
        let ty: Type = content.parse().map_err(|error: syn::Error| {
            syn::Error::new(
                error.span(),
                format!(
                    "expected a valid closing tag for component\nnote: found opening tag `{lt}{0}{gt}`\nhelp: try `{lt}/{0}{gt}`",
                    format_token_stream(&open.ty),
                    lt = open.tag.lt.to_token_stream(),
                    gt = open.tag.gt.to_token_stream(),
                ),
            )
        })?;
        if ty != open.ty && !is_ide_completion() {
            let open_ty = &open.ty;
            return Err(syn::Error::new_spanned(
                quote!(#open_ty #ty),
                format!(
                    "mismatched closing tags: expected `{}`, found `{}`",
                    format_token_stream(open_ty),
                    format_token_stream(ty)
                ),
            ));
        }
        input.advance_to(&input_fork);
        Ok(HtmlComponentClose { tag, ty })
    })
}

fn parse_component_children(
    input: ParseStream,
    open: &HtmlComponentOpen,
) -> syn::Result<(HtmlChildrenTree, Option<HtmlComponentClose>)> {
    let mut children = HtmlChildrenTree::new();
    loop {
        if input.is_empty() {
            if is_ide_completion() {
                return Ok((children, None));
            }
            return Err(syn::Error::new_spanned(
                open.to_spanned(),
                "this opening tag has no corresponding closing tag",
            ));
        }
        if is_component_close(input) {
            return parse_component_close(input, open).map(|close| (children, Some(close)));
        }
        children.parse_child(input)?;
    }
}

fn validate_component_children(
    open: &HtmlComponentOpen,
    children: &HtmlChildrenTree,
) -> syn::Result<()> {
    if children.is_empty() {
        return Ok(());
    }
    match open.props.children() {
        Some(children_prop) => Err(syn::Error::new_spanned(
            &children_prop.label,
            "cannot specify the `children` prop when the component already has children",
        )),
        None => Ok(()),
    }
}

pub struct HtmlComponent {
    ty: Type,
    pub props: ComponentProps,
    children: HtmlChildrenTree,
    close: Option<HtmlComponentClose>,
}

impl Parse for HtmlComponent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        reject_orphan_component_close(input)?;

        let open = input.parse::<HtmlComponentOpen>()?;
        // Return early if it's a self-closing tag
        if open.is_self_closing() {
            return Ok(HtmlComponent {
                ty: open.ty,
                props: open.props,
                children: HtmlChildrenTree::new(),
                close: None,
            });
        }

        let (children, close) = parse_component_children(input, &open)?;
        validate_component_children(&open, &children)?;

        Ok(HtmlComponent {
            ty: open.ty,
            props: open.props,
            children,
            close,
        })
    }
}

impl ToTokens for HtmlComponent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            ty,
            props,
            children,
            close,
        } = self;

        let ty_span = ty.span().resolved_at(Span::call_site());
        let props_ty = quote_spanned!(ty_span=> <#ty as ::yew::html::BaseComponent>::Properties);
        let children_renderer = children.to_children_renderer_tokens();
        let build_props = props.build_properties_tokens(&props_ty, children_renderer);
        let key = props.special().wrap_key_attr();
        let use_close_tag = close
            .as_ref()
            .map(|close| {
                let close_ty = &close.ty;
                quote_spanned! {close_ty.span()=>
                    let _ = |_:#close_ty| {};
                }
            })
            .unwrap_or_default();

        tokens.extend(quote_spanned! {ty_span=>
            {
                #use_close_tag
                #[allow(clippy::let_unit_value)]
                let __yew_props = #build_props;
                ::yew::virtual_dom::VChild::<#ty>::new(__yew_props, #key)
            }
        });
    }
}

struct HtmlComponentOpen {
    tag: TagTokens,
    ty: Type,
    props: ComponentProps,
}
impl HtmlComponentOpen {
    fn is_self_closing(&self) -> bool {
        self.tag.div.is_some()
    }

    fn to_spanned(&self) -> impl ToTokens {
        self.tag.to_spanned()
    }
}

impl Parse for HtmlComponentOpen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        TagTokens::parse_start_content(input, |input, tag| {
            let ty = input.parse()?;
            let props: ComponentProps = input.parse()?;

            if let Some(ref node_ref) = props.special().node_ref {
                return Err(syn::Error::new_spanned(
                    &node_ref.label,
                    "cannot use `ref` with components. If you want to specify a property, use \
                     `r#ref` here instead.",
                ));
            }

            Ok(Self { tag, ty, props })
        })
    }
}

struct HtmlComponentClose {
    tag: TagTokens,
    ty: Type,
}
impl HtmlComponentClose {
    fn to_spanned(&self) -> impl ToTokens {
        self.tag.to_spanned()
    }
}

impl Parse for HtmlComponentClose {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        TagTokens::parse_end_content(input, |input, tag| {
            let ty = input.parse()?;
            Ok(Self { tag, ty })
        })
    }
}
