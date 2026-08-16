use proc_macro2::{Delimiter, Group, Span, TokenStream};
use proc_macro_error3::emit_warning;
use quote::{quote, quote_spanned, ToTokens};
use syn::buffer::Cursor;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Lit, LitStr, Token};

use super::{HtmlChildrenTree, HtmlDashedName, TagTokens};
use crate::props::{ElementProps, Prop, PropDirective};
use crate::stringify::{Stringify, Value};
use crate::{is_ide_completion, non_capitalized_ascii, Peek, PeekValue};

fn is_normalised_element_name(name: &str) -> bool {
    match name {
        "animateMotion"
        | "animateTransform"
        | "clipPath"
        | "feBlend"
        | "feColorMatrix"
        | "feComponentTransfer"
        | "feComposite"
        | "feConvolveMatrix"
        | "feDiffuseLighting"
        | "feDisplacementMap"
        | "feDistantLight"
        | "feDropShadow"
        | "feFlood"
        | "feFuncA"
        | "feFuncB"
        | "feFuncG"
        | "feFuncR"
        | "feGaussianBlur"
        | "feImage"
        | "feMerge"
        | "feMergeNode"
        | "feMorphology"
        | "feOffset"
        | "fePointLight"
        | "feSpecularLighting"
        | "feSpotLight"
        | "feTile"
        | "feTurbulence"
        | "foreignObject"
        | "glyphRef"
        | "linearGradient"
        | "radialGradient"
        | "textPath" => true,
        _ => !name.chars().any(|c| c.is_ascii_uppercase()),
    }
}

pub struct HtmlElement {
    pub name: TagName,
    pub props: ElementProps,
    pub children: HtmlChildrenTree,
}

fn reject_orphan_element_close(input: ParseStream) -> syn::Result<()> {
    if HtmlElementClose::peek(input.cursor()).is_none() {
        return Ok(());
    }
    match input.parse::<HtmlElementClose>() {
        Ok(close) => Err(syn::Error::new_spanned(
            close.to_spanned(),
            "this closing tag has no corresponding opening tag",
        )),
        Err(error) => Err(error),
    }
}

fn validate_element_open(open: &HtmlElementOpen) -> syn::Result<()> {
    let TagName::Lit(name) = &open.name else {
        return Ok(());
    };
    match name.to_ascii_lowercase_string().as_str() {
        "textarea" => Err(syn::Error::new_spanned(
            open.to_spanned(),
            "the tag `<textarea>` is a void element and cannot have children (hint: to provide value to it, rewrite it as `<textarea value={x} />`. If you wish to set the default value, rewrite it as `<textarea defaultvalue={x} />`)",
        )),
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link"
        | "meta" | "param" | "source" | "track" | "wbr" => Err(syn::Error::new_spanned(
            open.to_spanned(),
            format!(
                "the tag `<{name}>` is a void element and cannot have children (hint: rewrite this as `<{name} />`)",
            ),
        )),
        _ => Ok(()),
    }
}

fn parse_element_children(
    input: ParseStream,
    open: &HtmlElementOpen,
) -> syn::Result<HtmlChildrenTree> {
    let open_key = open.name.get_key();
    let mut children = HtmlChildrenTree::new();
    loop {
        if input.is_empty() {
            if is_ide_completion() {
                return Ok(children);
            }
            return Err(syn::Error::new_spanned(
                open.to_spanned(),
                "this opening tag has no corresponding closing tag",
            ));
        }
        if HtmlElementClose::peek(input.cursor()).is_some_and(|close_key| open_key == close_key) {
            break;
        }
        children.parse_child(input)?;
    }
    if !input.is_empty() || !is_ide_completion() {
        input.parse::<HtmlElementClose>()?;
    }
    Ok(children)
}

impl PeekValue<()> for HtmlElement {
    fn peek(cursor: Cursor) -> Option<()> {
        HtmlElementOpen::peek(cursor)
            .or_else(|| HtmlElementClose::peek(cursor))
            .map(|_| ())
    }
}

impl Parse for HtmlElement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        reject_orphan_element_close(input)?;

        let open = input.parse::<HtmlElementOpen>()?;
        // Return early if it's a self-closing tag
        if open.is_self_closing() {
            return Ok(HtmlElement {
                name: open.name,
                props: open.props,
                children: HtmlChildrenTree::new(),
            });
        }

        validate_element_open(&open)?;
        let children = parse_element_children(input, &open)?;

        Ok(Self {
            name: open.name,
            props: open.props,
            children,
        })
    }
}

type Attribute = (LitStr, Value, Option<PropDirective>);

fn special_attribute(prop: Option<&Prop>) -> TokenStream {
    prop.map(|prop| wrap_attr_value(prop.value.optimize_literals()))
        .unwrap_or(quote! { ::std::option::Option::None })
}

fn checked_attribute(prop: Option<&Prop>) -> TokenStream {
    prop.map(|prop| {
        let value = &prop.value;
        quote! { ::std::option::Option::Some( #value ) }
    })
    .unwrap_or(quote! { ::std::option::Option::None })
}

fn boolean_attribute(prop: &Prop) -> Option<Attribute> {
    let key = prop.label.to_lit_str();
    let expression = &prop.value;
    let value = match &prop.value {
        Expr::Lit(expr) => match &expr.lit {
            Lit::Bool(boolean) if !boolean.value => return None,
            Lit::Bool(_) => Value::Static(quote! { #key }),
            _ => Value::Dynamic(quote_spanned! {expression.span()=> {
                ::yew::utils::__ensure_type::<::std::primitive::bool>(#expression);
                #key
            }}),
        },
        expr => Value::Dynamic(quote_spanned! {expr.span().resolved_at(Span::call_site())=>
            if #expr {
                ::std::option::Option::Some(
                    ::yew::virtual_dom::AttrValue::Static(#key)
                )
            } else {
                ::std::option::Option::None
            }
        }),
    };
    Some((key, value, prop.directive))
}

fn class_attribute(prop: Option<&Prop>) -> Option<Attribute> {
    let prop = prop?;
    match prop.value.try_into_lit() {
        Some(literal) if literal.value().is_empty() => None,
        Some(literal) => Some((
            LitStr::new("class", literal.span()),
            Value::Static(quote! { #literal }),
            None,
        )),
        None => {
            let expression = &prop.value;
            Some((
                LitStr::new("class", prop.label.span()),
                Value::Dynamic(quote! {
                    ::std::convert::Into::<::yew::html::Classes>::into(#expression)
                }),
                None,
            ))
        }
    }
}

fn static_attributes(attributes: &[Attribute]) -> Option<TokenStream> {
    if attributes
        .iter()
        .any(|(_, _, directive)| matches!(directive, Some(PropDirective::ApplyAsProperty(_))))
    {
        return None;
    }
    let mut pairs = Vec::with_capacity(attributes.len());
    for (key, value, _) in attributes {
        let Value::Static(value) = value else {
            return None;
        };
        pairs.push(quote! {
            (
                #key,
                ::yew::virtual_dom::AttributeOrProperty::Static(#value)
            )
        });
    }
    Some(quote! { ::yew::virtual_dom::Attributes::Static(&[#(#pairs),*]) })
}

fn dynamic_attributes(attributes: &[Attribute]) -> TokenStream {
    let keys = attributes.iter().map(|(key, ..)| quote! { #key });
    let values = attributes.iter().map(|(_, value, directive)| {
        match directive {
            Some(PropDirective::ApplyAsProperty(token)) => {
                quote_spanned!(token.span()=> ::std::option::Option::Some(
                    ::yew::virtual_dom::AttributeOrProperty::Property(
                        ::std::convert::Into::into(#value)
                    )
                ))
            }
            None => {
                let value = wrap_attr_value(value);
                quote! {
                    ::std::option::Option::map(#value, ::yew::virtual_dom::AttributeOrProperty::Attribute)
                }
            }
        }
    });
    quote! {
        ::yew::virtual_dom::Attributes::Dynamic{
            keys: &[#(#keys),*],
            values: ::std::boxed::Box::new([#(#values),*]),
        }
    }
}

fn build_attributes(props: &ElementProps) -> TokenStream {
    let normal = props.attributes.iter().map(|prop| {
        (
            prop.label.to_lit_str(),
            prop.value.optimize_literals_tagged(),
            prop.directive,
        )
    });
    let attributes = normal
        .chain(props.booleans.iter().filter_map(boolean_attribute))
        .chain(class_attribute(props.classes.as_ref()))
        .collect::<Vec<_>>();
    static_attributes(&attributes).unwrap_or_else(|| dynamic_attributes(&attributes))
}

fn build_listeners(props: &ElementProps) -> TokenStream {
    if props.listeners.is_empty() {
        return quote! { ::yew::virtual_dom::listeners::Listeners::None };
    }
    let listeners = props.listeners.iter().map(|prop| {
        let name = &prop.label.name;
        let value = &prop.value;
        quote! { ::yew::html::#name::Wrapper::__macro_new(#value) }
    });
    quote! {
        ::yew::virtual_dom::listeners::Listeners::Pending(
            ::std::boxed::Box::new([#(#listeners),*])
        )
    }
}

fn render_literal_element(
    dashed_name: &HtmlDashedName,
    props: &ElementProps,
    node_ref: &TokenStream,
    key: &TokenStream,
    attributes: &TokenStream,
    listeners: &TokenStream,
    children: &TokenStream,
) -> TokenStream {
    let name_span = dashed_name.span();
    let name = dashed_name.to_string();
    let lowercase_name = dashed_name.to_ascii_lowercase_string();
    if !is_normalised_element_name(&name) {
        emit_warning!(
            name_span.clone(),
            format!(
                "The tag '{dashed_name}' is not matching its normalized form '{lowercase_name}' and is not a recognized SVG or MathML element. If you want to keep this name, you can use the dynamic tag `@{{\"{dashed_name}\"}}` to silence this warning."
            )
        )
    }
    let node = match lowercase_name.as_str() {
        "input" => {
            let value = special_attribute(props.value.as_ref());
            let checked = checked_attribute(props.checked.as_ref());
            quote! {
                ::std::convert::Into::<::yew::virtual_dom::VNode>::into(
                    ::yew::virtual_dom::VTag::__new_input(
                        #value, #checked, #node_ref, #key, #attributes, #listeners,
                    ),
                )
            }
        }
        "textarea" => {
            let value = special_attribute(props.value.as_ref());
            let defaultvalue = special_attribute(props.defaultvalue.as_ref());
            quote! {
                ::std::convert::Into::<::yew::virtual_dom::VNode>::into(
                    ::yew::virtual_dom::VTag::__new_textarea(
                        #value, #defaultvalue, #node_ref, #key, #attributes, #listeners,
                    ),
                )
            }
        }
        _ => quote! {
            ::std::convert::Into::<::yew::virtual_dom::VNode>::into(
                ::yew::virtual_dom::VTag::__new_other(
                    ::yew::virtual_dom::AttrValue::Static(#name),
                    #node_ref, #key, #attributes, #listeners, #children,
                ),
            )
        },
    };
    quote_spanned! {name_span=> {
        #[allow(clippy::redundant_clone, unused_braces)]
        let node = #node;
        node
    }}
}

#[rustversion::since(1.88)]
fn derive_debug_tag(vtag: &Ident) -> String {
    let span = vtag.span().unwrap();
    format!("[{}:{}:{}] ", span.file(), span.line(), span.column())
}

#[rustversion::before(1.88)]
fn derive_debug_tag(_: &Ident) -> &'static str {
    ""
}

fn render_dynamic_element(
    name: &DynamicName,
    props: &ElementProps,
    node_ref: &TokenStream,
    key: &TokenStream,
    attributes: &TokenStream,
    listeners: &TokenStream,
    children: &TokenStream,
) -> TokenStream {
    let vtag = Ident::new("__yew_vtag", name.span());
    let expr = name.expr.as_ref().map(Group::stream);
    let vtag_name = Ident::new("__yew_vtag_name", expr.span());
    let void_children = Ident::new("__yew_void_children", Span::mixed_site());
    let handle_value_attr = props.value.as_ref().map(|prop| {
        let value = prop.value.optimize_literals();
        quote_spanned! {value.span()=> {
            __yew_vtag.__macro_push_attr("value", #value);
        }}
    });
    let invalid_void_tag_msg_start = derive_debug_tag(&vtag);
    let value = special_attribute(props.value.as_ref());
    let checked = checked_attribute(props.checked.as_ref());
    let defaultvalue = special_attribute(props.defaultvalue.as_ref());
    quote_spanned! {expr.span()=> {
                    let mut #vtag_name = ::std::convert::Into::<
                        ::yew::virtual_dom::AttrValue
                    >::into(#expr);
                    ::std::debug_assert!(
                        #vtag_name.is_ascii(),
                        "a dynamic tag returned a tag name containing non ASCII characters: `{}`",
                        #vtag_name,
                    );

                    #[allow(clippy::redundant_clone, unused_braces, clippy::let_and_return)]
                    let mut #vtag = match () {
                        _ if "input".eq_ignore_ascii_case(::std::convert::AsRef::<::std::primitive::str>::as_ref(&#vtag_name)) => {
                            ::yew::virtual_dom::VTag::__new_input(
                                #value,
                                #checked,
                                #node_ref,
                                #key,
                                #attributes,
                                #listeners,
                            )
                        }
                        _ if "textarea".eq_ignore_ascii_case(::std::convert::AsRef::<::std::primitive::str>::as_ref(&#vtag_name)) => {
                            ::yew::virtual_dom::VTag::__new_textarea(
                                #value,
                                #defaultvalue,
                                #node_ref,
                                #key,
                                #attributes,
                                #listeners,
                            )
                        }
                        _ => {
                            let mut __yew_vtag = ::yew::virtual_dom::VTag::__new_other(
                                #vtag_name,
                                #node_ref,
                                #key,
                                #attributes,
                                #listeners,
                                #children,
                            );

                            #handle_value_attr

                            __yew_vtag
                        }
                    };

                    // These are the runtime-checks exclusive to dynamic tags.
                    // For literal tags this is already done at compile-time.
                    //
                    // check void element
                    if ::yew::virtual_dom::VTag::children(&#vtag).is_some() &&
                       !::std::matches!(
                        ::yew::virtual_dom::VTag::children(&#vtag),
                        ::std::option::Option::Some(::yew::virtual_dom::VNode::VList(ref #void_children)) if ::std::vec::Vec::is_empty(#void_children)
                    ) {
                        ::std::debug_assert!(
                            !::std::matches!(#vtag.tag().to_ascii_lowercase().as_str(),
                                "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input"
                                    | "link" | "meta" | "param" | "source" | "track" | "wbr" | "textarea"
                            ),
                            concat!(#invalid_void_tag_msg_start, "a dynamic tag tried to create a `<{0}>` tag with children. `<{0}>` is a void element which can't have any children."),
                            #vtag.tag(),
                        );
                    }

                    ::std::convert::Into::<::yew::virtual_dom::VNode>::into(#vtag)
    }}
}

impl ToTokens for HtmlElement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let node_ref = self.props.special.wrap_node_ref_attr();
        let key = self.props.special.wrap_key_attr();
        let attributes = build_attributes(&self.props);
        let listeners = build_listeners(&self.props);
        let children = self.children.to_vnode_tokens();
        let element = match &self.name {
            TagName::Lit(name) => render_literal_element(
                name,
                &self.props,
                &node_ref,
                &key,
                &attributes,
                &listeners,
                &children,
            ),
            TagName::Expr(name) => render_dynamic_element(
                name,
                &self.props,
                &node_ref,
                &key,
                &attributes,
                &listeners,
                &children,
            ),
        };
        tokens.extend(element);
    }
}

fn wrap_attr_value<T: ToTokens>(value: T) -> TokenStream {
    quote_spanned! {value.span()=>
        ::yew::html::IntoPropValue::<
            ::std::option::Option<
                ::yew::virtual_dom::AttrValue
            >
        >
        ::into_prop_value(#value)
    }
}

pub struct DynamicName {
    at: Token![@],
    expr: Option<Group>,
}

impl Peek<'_, ()> for DynamicName {
    fn peek(cursor: Cursor) -> Option<((), Cursor)> {
        let (punct, cursor) = cursor.punct()?;
        if punct.as_char() != '@' {
            return None;
        }

        // move cursor past block if there is one
        let cursor = cursor
            .group(Delimiter::Brace)
            .map(|(_, _, cursor)| cursor)
            .unwrap_or(cursor);

        Some(((), cursor))
    }
}

impl Parse for DynamicName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let at = input.parse()?;
        // the expression block is optional, closing tags don't have it.
        let expr = input.parse().ok();
        Ok(Self { at, expr })
    }
}

impl ToTokens for DynamicName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { at, expr } = self;
        tokens.extend(quote! {#at #expr});
    }
}

#[derive(PartialEq)]
enum TagKey {
    Lit(HtmlDashedName),
    Expr,
}

pub enum TagName {
    Lit(HtmlDashedName),
    Expr(DynamicName),
}

impl TagName {
    fn get_key(&self) -> TagKey {
        match self {
            TagName::Lit(name) => TagKey::Lit(name.clone()),
            TagName::Expr(_) => TagKey::Expr,
        }
    }
}

impl Peek<'_, TagKey> for TagName {
    fn peek(cursor: Cursor) -> Option<(TagKey, Cursor)> {
        if let Some((_, cursor)) = DynamicName::peek(cursor) {
            Some((TagKey::Expr, cursor))
        } else {
            HtmlDashedName::peek(cursor).map(|(name, cursor)| (TagKey::Lit(name), cursor))
        }
    }
}

impl Parse for TagName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if DynamicName::peek(input.cursor()).is_some() {
            DynamicName::parse(input).map(Self::Expr)
        } else {
            HtmlDashedName::parse(input).map(Self::Lit)
        }
    }
}

impl ToTokens for TagName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            TagName::Lit(name) => name.to_tokens(tokens),
            TagName::Expr(name) => name.to_tokens(tokens),
        }
    }
}

struct HtmlElementOpen {
    tag: TagTokens,
    name: TagName,
    props: ElementProps,
}
impl HtmlElementOpen {
    fn is_self_closing(&self) -> bool {
        self.tag.div.is_some()
    }

    fn to_spanned(&self) -> impl ToTokens {
        self.tag.to_spanned()
    }
}

impl PeekValue<TagKey> for HtmlElementOpen {
    fn peek(cursor: Cursor) -> Option<TagKey> {
        let (punct, cursor) = cursor.punct()?;
        if punct.as_char() != '<' {
            return None;
        }

        let (tag_key, cursor) = TagName::peek(cursor)?;
        if let TagKey::Lit(name) = &tag_key {
            // Avoid parsing `<key=[...]>` as an element. It needs to be parsed as an `HtmlList`.
            if name.to_string() == "key" {
                let (punct, _) = cursor.punct()?;
                // ... unless it isn't followed by a '='. `<key></key>` is a valid element!
                if punct.as_char() == '=' {
                    return None;
                }
            } else if !non_capitalized_ascii(&name.to_string()) {
                return None;
            }
        }

        Some(tag_key)
    }
}

impl Parse for HtmlElementOpen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        TagTokens::parse_start_content(input, |input, tag| {
            let name = input.parse::<TagName>()?;
            let mut props = input.parse::<ElementProps>()?;
            normalize_element_props(&name, &mut props)?;
            Ok(Self { tag, name, props })
        })
    }
}

fn normalize_element_props(name: &TagName, props: &mut ElementProps) -> syn::Result<()> {
    match name {
        TagName::Lit(name)
            if !matches!(
                name.to_ascii_lowercase_string().as_str(),
                "input" | "textarea"
            ) =>
        {
            props.attributes.extend(props.value.take());
            props.attributes.extend(props.checked.take());
            Ok(())
        }
        TagName::Expr(name) if name.expr.is_none() => Err(syn::Error::new_spanned(
            name,
            "this dynamic tag is missing an expression block defining its value",
        )),
        _ => Ok(()),
    }
}

struct HtmlElementClose {
    tag: TagTokens,
    _name: TagName,
}
impl HtmlElementClose {
    fn to_spanned(&self) -> impl ToTokens {
        self.tag.to_spanned()
    }
}

impl PeekValue<TagKey> for HtmlElementClose {
    fn peek(cursor: Cursor) -> Option<TagKey> {
        let (punct, cursor) = cursor.punct()?;
        if punct.as_char() != '<' {
            return None;
        }

        let (punct, cursor) = cursor.punct()?;
        if punct.as_char() != '/' {
            return None;
        }

        let (tag_key, cursor) = TagName::peek(cursor)?;
        if matches!(&tag_key, TagKey::Lit(name) if !non_capitalized_ascii(&name.to_string())) {
            return None;
        }

        let (punct, _) = cursor.punct()?;
        (punct.as_char() == '>').then_some(tag_key)
    }
}

impl Parse for HtmlElementClose {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        TagTokens::parse_end_content(input, |input, tag| {
            let name = input.parse()?;

            if let TagName::Expr(name) = &name {
                if let Some(expr) = &name.expr {
                    return Err(syn::Error::new_spanned(
                        expr,
                        "dynamic closing tags must not have a body (hint: replace it with just \
                         `</@>`)",
                    ));
                }
            }

            Ok(Self { tag, _name: name })
        })
    }
}
