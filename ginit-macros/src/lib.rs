extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn main(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let name = parse_macro_input!(args as syn::Expr);
    let exec_struct = parse_macro_input!(input as syn::ItemStruct);
    let exec = &exec_struct.ident;
    let expanded = quote! {
        fn main() {
            ginit_core::util::cli::NonZeroExit::exec::<#exec>(#name)
        }

        #exec_struct
    };
    expanded.into()
}
