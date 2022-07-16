use proc_macro::TokenStream;

mod program;

#[proc_macro_attribute]
pub fn program(attr: TokenStream, item: TokenStream) -> TokenStream {
    program::expand(attr.into(), item.into()).into()
}
