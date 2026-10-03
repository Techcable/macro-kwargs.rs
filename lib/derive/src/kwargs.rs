use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Error, Fields, LitStr, Path, Token, Type};

pub fn run_derive(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let s = match input.data {
        Data::Struct(ref s) => s,
        Data::Enum(ref e) => {
            return Err(Error::new(
                e.enum_token.span(),
                "Enums are currently unsupported",
            ));
        }
        Data::Union(ref u) => {
            return Err(Error::new(u.union_token.span(), "Unions are unsupported"));
        }
    };
    let named_fields = match s.fields {
        Fields::Named(ref named) => named,
        Fields::Unnamed(ref s) => {
            return Err(Error::new(s.span(), "Unnamed fields are forbidden"));
        }
        Fields::Unit => return Err(Error::new(input.ident.span(), "Unit structs are forbidden")),
    };
    let original_ident = &input.ident;
    let original_vis = &input.vis;
    let id_enum_name = format_ident!("{}ArgId", input.ident);
    let parsed_arg_name = format_ident!("{}ParsedArg", input.ident);
    let mut variant_names = Vec::new();
    let mut field_declarations = Vec::new();
    let mut field_inits = Vec::new();
    let mut parsed_arg_types = Vec::new();
    let mut arg_name_strings = Vec::new();
    let mut parse_invocations = Vec::new();
    for field in &named_fields.named {
        use heck::ToUpperCamelCase;
        let mut attr = FieldAttrs::find_attr(&field.attrs)?.unwrap_or_default();
        let ident = field.ident.as_ref().unwrap();
        let arg_name = attr.rename.clone().unwrap_or_else(|| ident.to_string());
        let variant_name = Ident::new(&ident.to_string().to_upper_camel_case(), ident.span());
        variant_names.push(variant_name.clone());
        arg_name_strings.push(arg_name);
        parsed_arg_types.push(&field.ty);
        let field_ty = &field.ty;
        // For now, only used for `with_syn`
        let mut custom_with_wrapper_conversion = None;
        // implement `with_syn` by reducing to `with_wrapper` with custom conversion
        if std::mem::replace(&mut attr.with_syn, false) {
            attr.with_wrapper = Some(syn::parse_quote_spanned! {
                field_ty.span() => macro_kwargs::parse::Syn::<#field_ty>
            });
            custom_with_wrapper_conversion = Some(quote!(wrapper.into_inner()));
        }
        match (attr.with_func.as_ref(), attr.with_wrapper.as_ref()) {
            (Some(_), Some(_)) => unreachable!("conflicting 'with' options"),
            (Some(with_func), None) => {
                parse_invocations.push(quote_spanned!(
                    with_func.span() => #with_func(stream)?
                ));
            }
            (None, Some(wrapper_ty)) => {
                let with_wrapper_conversion = custom_with_wrapper_conversion.unwrap_or_else(
                    || quote!(<#wrapper_ty as core::convert::Into::<#field_ty>>::into(wrapper)),
                );
                parse_invocations.push(quote_spanned!(
                    wrapper_ty.span() => {
                        let wrapper = <#wrapper_ty as macro_kwargs::MacroArg>::parse_macro_arg(stream)?;
                        #with_wrapper_conversion
                    }
                ));
            }
            (None, None) => {
                parse_invocations.push(quote_spanned!(
                    field.ty.span() => <#field_ty as macro_kwargs::MacroArg>::parse_macro_arg(stream)?
                ));
            }
        }
        /*
         * In order to produce good errors,
         * field initialization has three phases:
         * 1. Declaration
         * 2. Checking for errors (if any required args were missing)
         * 3. Struct Initialization
         *
         * Splitting it up allows us to combine multiple error
         * messages for missing required arguments.
         */
        let cast_failure = quote!(unreachable!(
            "got {:?} for {}",
            other.id(),
            stringify!(#variant_name)
        ));
        if attr.optional {
            let default_val = quote_spanned!(
                field.ty.span() => <#field_ty as Default>::default()
            );
            field_declarations.push(quote! {
                let #ident: #field_ty = match argument_list
                    .take(#id_enum_name::#variant_name)
                    .map(|arg| arg.value) {
                    Some(#parsed_arg_name::#variant_name ( res )) => res,
                    Some(other) => #cast_failure,
                    None => #default_val
                };
            });
            field_inits.push(quote!(#ident));
        } else {
            field_declarations.push(quote! {
                let #ident: Option<#field_ty> = match argument_list.require(#id_enum_name::#variant_name).map(|arg| arg.value) {
                    Ok(#parsed_arg_name::#variant_name ( res ) ) => Some(res),
                    Ok(other) => #cast_failure,
                    Err(e) => {
                        missing_argument_errors.push(e);
                        None // temporary - will never be used
                    }
                };
            });
            field_inits.push(quote!(#ident: #ident.unwrap()));
        }
    }
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
        #[doc(hidden)]
        #original_vis enum #id_enum_name {
            #(#variant_names),*
        }
        #[automatically_derived]
        impl macro_kwargs::args::KeywordArgId for #id_enum_name {
            fn as_str(&self) -> &'_ str {
                match *self {
                    #(Self::#variant_names => #arg_name_strings),*
                }
            }
            fn from_name(name: &str) -> Option<Self> {
                match name {
                    #(#arg_name_strings => Some(Self::#variant_names),)*
                    _ => None
                }
            }
        }
        #[doc(hidden)]
        #original_vis enum #parsed_arg_name {
            #(#variant_names ( #parsed_arg_types )),*
        }
        #[automatically_derived]
        impl macro_kwargs::args::ParsedArgValue<#id_enum_name> for #parsed_arg_name {
            fn id(&self) -> #id_enum_name {
                match *self {
                    #(#parsed_arg_name::#variant_names (_) => #id_enum_name::#variant_names),*
                }
            }
            fn parse_with_id(
                id: #id_enum_name ,
                id_span: proc_macro2::Span,
                stream: syn::parse::ParseStream,
            ) -> syn::Result<Self> {
                Ok(match id {
                    #(#id_enum_name::#variant_names => {
                        Self::#variant_names(#parse_invocations)
                    }),*
                })
            }
        }
        #[automatically_derived]
        impl #impl_generics macro_kwargs::MacroKeywordArgs
                for #original_ident #ty_generics #where_clause {
            type ArgId = #id_enum_name;
            type ParsedArg = #parsed_arg_name;
            fn from_keyword_args(mut argument_list: macro_kwargs::args::ParsedKeywordArguments<Self>) -> syn::Result<Self> {
                #[allow(unused_imports)] // Possible if empty
                use macro_kwargs::args::ParsedArgValue;
                let mut missing_argument_errors = Vec::new();
                #(#field_declarations)*
                if !missing_argument_errors.is_empty() {
                    return Err(macro_kwargs::combine_errors(missing_argument_errors));
                }
                Ok(#original_ident {
                    #(#field_inits),*
                })
            }
        }
        #[automatically_derived]
        impl #impl_generics syn::parse::Parse
                for #original_ident #ty_generics #where_clause {
            fn parse(stream: syn::parse::ParseStream) -> syn::Result<Self> {
                Self::from_keyword_args(stream.parse()?)
            }
        }
        /// Parse as a nested value inside another set of arguments,
        /// by surrounding it with braces `{}`
        #[automatically_derived]
        impl #impl_generics macro_kwargs::MacroArg
                for #original_ident #ty_generics #where_clause {
            fn parse_macro_arg(stream: syn::parse::ParseStream) -> syn::Result<Self> {
                Self::from_keyword_args(
                    <macro_kwargs::args::ParsedKeywordArguments<Self> as macro_kwargs::MacroArg>::parse_macro_arg(stream)?
                )
            }
        }
    })
}

struct FieldAttrs {
    /// If this field is optional,
    /// and should be replaced with its `Default`
    /// value if missing
    optional: bool,
    /// Rename the field's expected.
    rename: Option<String>,
    /// Parse the value by delegating to the specified function
    ///
    /// The function's signature must be
    /// `fn(ParseStream) -> syn::Result<T>`
    with_func: Option<Path>,
    /// Parse the value by delegating to the specified "wrapper"
    /// type, then converts it to the actual type via `Into`
    with_wrapper: Option<Type>,
    /// If true, parses using the syn [`syn::Parse`] trait.
    ///
    /// Equivalent to `with_wrapper = macro_kwargs::parse::Syn`.
    ///
    /// [`syn::Parse`]: https://docs.rs/syn/latest/syn/parse/trait.Parse.html
    with_syn: bool,
}
#[allow(clippy::derivable_impls)]
impl Default for FieldAttrs {
    fn default() -> Self {
        FieldAttrs {
            optional: false,
            rename: None,
            with_func: None,
            with_wrapper: None,
            with_syn: false,
        }
    }
}
impl FieldAttrs {
    fn find_attr(attrs: &[Attribute]) -> syn::Result<Option<Self>> {
        let mut res = None;
        for attr in attrs {
            if attr.path().is_ident("kwarg") {
                if res.is_some() {
                    return Err(Error::new(
                        attr.path().span(),
                        "Duplicate `kwarg` attributes",
                    ));
                }
                res = Some(attr.parse_args::<Self>()?);
            }
        }
        Ok(res)
    }
}
impl Parse for FieldAttrs {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        #[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
        enum Attr {
            Optional,
            Rename,
            WithFunc,
            WithWrapper,
            WithSyn,
        }
        macro_rules! declare_names {
            ($($variant:ident => [$primary:literal $(, $($secondary:literal),*)?]),+ $(,)?) => {
                impl Attr {
                    fn name(self) -> &'static str {
                        match self {
                            $(Attr::$variant => $primary,)*
                        }
                    }
                    fn from_name(s: &str) -> Option<Self> {
                        match s {
                            $($primary $($(| $secondary)*)* => Some(Attr::$variant),)*
                            _ => None,
                        }
                    }
                }
            }
        }
        declare_names! {
            Optional => ["optional"],
            Rename => ["rename"],
            WithFunc => ["with_func"],
            WithWrapper => ["with_wrapper"],
            WithSyn => ["with_syn", "syn"],
        }
        impl Attr {
            fn is_with_opt(self) -> bool {
                matches!(self, Attr::WithFunc | Attr::WithWrapper | Attr::WithSyn)
            }
            fn conflicts_with(self, other: Attr) -> bool {
                self.is_with_opt() && other.is_with_opt()
            }
        }

        let mut res = Self::default();
        // using BTreeSet gives deterministic errors
        let mut existing_opts = BTreeSet::new();
        loop {
            let name: Ident = stream.parse()?;
            let opt = Attr::from_name(&name.to_string())
                .ok_or_else(|| Error::new(name.span(), "Unknown option name"))?;
            // check for conflicts & duplicates
            if existing_opts.contains(&opt) {
                return Err(Error::new(
                    name.span(),
                    format!("Already specified `{}` attribute", opt.name()),
                ));
            }
            for &other in &existing_opts {
                if opt.conflicts_with(other) {
                    return Err(Error::new(
                        name.span(),
                        format!(
                            "The `{}` option conflicts with the `{}` option",
                            opt.name(),
                            other.name(),
                        ),
                    ));
                }
            }
            // now that we've verified there are no conflicts, add it to the set
            assert!(existing_opts.insert(opt));

            match opt {
                Attr::Optional => {
                    res.optional = true;
                }
                Attr::Rename => {
                    stream.parse::<Token![=]>()?;
                    let renamed = stream.parse::<LitStr>()?;
                    res.rename = Some(renamed.value());
                }
                Attr::WithFunc => {
                    stream.parse::<Token![=]>()?;
                    let s = stream.parse::<LitStr>()?;
                    res.with_func = Some(s.parse::<syn::Path>()?);
                }
                Attr::WithWrapper => {
                    stream.parse::<Token![=]>()?;
                    let s = stream.parse::<LitStr>()?;
                    res.with_wrapper = Some(s.parse::<Type>()?);
                }
                Attr::WithSyn => {
                    res.with_syn = true;
                }
            }
            if stream.peek(Token![,]) {
                stream.parse::<Token![,]>()?;
                continue;
            } else {
                break;
            }
        }
        if !stream.is_empty() {
            return Err(stream.error("Unexpected token"));
        }
        Ok(res)
    }
}
