use std::collections::HashSet;

use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input, parse_quote,
};

struct YesOrNo {
    vis: syn::Visibility,
    name: syn::Ident,
    extra_derives: syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
}

impl Parse for YesOrNo {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse()?;
        let name = input.parse()?;
        let _comma: Option<syn::token::Comma> = input.parse()?;
        let extra_derives = input.parse_terminated(syn::Expr::parse)?;
        Ok(Self {
            vis,
            name,
            extra_derives,
        })
    }
}

#[proc_macro]
pub fn yes_or_no(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let YesOrNo {
        vis,
        name,
        extra_derives,
    } = parse_macro_input!(input as YesOrNo);
    let derives = {
        let default_derives: syn::punctuated::Punctuated<syn::Expr, syn::token::Comma> =
            parse_quote! {Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd};
        let mut derive_set = HashSet::new();
        let mut derives = syn::punctuated::Punctuated::<syn::Expr, syn::token::Comma>::new();
        for default_derive in default_derives.into_iter() {
            if !derive_set.contains(&default_derive) {
                derives.push(default_derive.clone());
            }
            derive_set.insert(default_derive);
        }
        for extra_derive in extra_derives.into_iter() {
            if !derive_set.contains(&extra_derive) {
                derives.push(extra_derive.clone());
            }
            derive_set.insert(extra_derive);
        }
        derives
    };

    let expanded = quote! {
        #[derive(#derives)]
        #vis enum #name {
            No,
            Yes,
        }

        impl #name {
            pub const fn from_bool(flag: bool) -> Self {
                if flag {
                    Self::Yes
                } else {
                    Self::No
                }
            }

            pub const fn yes(self) -> bool {
                matches!(self, Self::Yes)
            }

            pub const fn no(self) -> bool {
                matches!(self, Self::No)
            }

            pub const fn and(self, other: Self) -> Self {
                Self::from_bool(self.yes() & other.yes())
            }

            pub const fn or(self, other: Self) -> Self {
                Self::from_bool(self.yes() | other.yes())
            }

            pub const fn xor(self, other: Self) -> Self {
                Self::from_bool(self.yes() ^ other.yes())
            }

            pub const fn not(self) -> Self {
                Self::from_bool(self.no())
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self::No
            }
        }

        impl From<bool> for #name {
            fn from(flag: bool) -> Self {
                Self::from_bool(flag)
            }
        }

        impl Into<bool> for #name {
            fn into(self) -> bool {
                self.yes()
            }
        }

        impl std::ops::BitAnd for #name {
            type Output = Self;

            fn bitand(self, other: Self) -> Self::Output {
                self.and(other)
            }
        }

        impl std::ops::BitAnd<bool> for #name {
            type Output = bool;

            fn bitand(self, other: bool) -> Self::Output {
                self.yes() & other
            }
        }

        impl std::ops::BitAnd<#name> for bool {
            type Output = bool;

            fn bitand(self, other: #name) -> Self::Output {
                self & other.yes()
            }
        }

        impl std::ops::BitAndAssign for #name {
            fn bitand_assign(&mut self, other: Self) {
                *self = self.and(other);
            }
        }

        impl std::ops::BitOr for #name {
            type Output = Self;

            fn bitor(self, other: Self) -> Self::Output {
                self.or(other)
            }
        }

        impl std::ops::BitOr<bool> for #name {
            type Output = bool;

            fn bitor(self, other: bool) -> Self::Output {
                self.yes() | other
            }
        }

        impl std::ops::BitOr<#name> for bool {
            type Output = bool;

            fn bitor(self, other: #name) -> Self::Output {
                self | other.yes()
            }
        }

        impl std::ops::BitOrAssign for #name {
            fn bitor_assign(&mut self, other: Self) {
                *self = self.or(other);
            }
        }

        impl std::ops::BitXor for #name {
            type Output = Self;

            fn bitxor(self, other: Self) -> Self::Output {
                self.xor(other)
            }
        }

        impl std::ops::BitXor<bool> for #name {
            type Output = bool;

            fn bitxor(self, other: bool) -> Self::Output {
                self.yes() ^ other
            }
        }

        impl std::ops::BitXor<#name> for bool {
            type Output = bool;

            fn bitxor(self, other: #name) -> Self::Output {
                self ^ other.yes()
            }
        }

        impl std::ops::BitXorAssign for #name {
            fn bitxor_assign(&mut self, other: Self) {
                *self = self.xor(other);
            }
        }

        impl std::ops::Not for #name {
            type Output = Self;

            fn not(self) -> Self::Output {
                self.not()
            }
        }
    };
    // panic!("{}", expanded.to_string());
    expanded.into()
}
