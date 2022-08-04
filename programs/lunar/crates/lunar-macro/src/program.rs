use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse2, ItemMod};

pub fn expand(attr: TokenStream2, input: TokenStream2) -> TokenStream2 {
    match expand_or_err(attr, input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

pub fn expand_or_err(attr: TokenStream2, input: TokenStream2) -> syn::Result<TokenStream2> {
    let program = Program::new(attr, input)?;
    Ok(program.expand())
}

struct Program {
    target: Ident,
    module: ItemMod,
}

impl Program {
    fn new(attr: TokenStream2, module: TokenStream2) -> syn::Result<Self> {
        let target = syn::parse2::<Ident>(attr)?;
        let module = parse2::<ItemMod>(module)?;
        Ok(Program { target, module })
    }

    fn expand(&self) -> TokenStream2 {
        let target = &self.target;
        let ident = &self.module.ident;
        let (_, items) = &self.module.content.as_ref().unwrap();
        quote! {
            pub mod #ident {
                #( #items )*
            }

            pub mod _lunar_ {
                use super::*;
                use std::{os::raw::{c_uchar, c_void}};

                use lunar::{LunarExitCode, Program, wee_alloc};

                #[global_allocator]
                static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

                #[no_mangle]
                pub extern "C" fn lunar_main(
                    entry_data_ptr: *mut c_uchar,
                    size: usize,
                ) -> i32 {
                    let prgm = #ident::#target::default();
                    lunar::lunar_core_main(entry_data_ptr, size, prgm)
                }

                #[no_mangle]
                pub extern "C" fn lunar_allocate(size: usize) -> *mut c_void {
                    lunar::lunar_core_allocate(size)
                }

                #[no_mangle]
                pub extern "C" fn lunar_deallocate(ptr: *mut c_void, size: usize) {
                    lunar::lunar_core_deallocate(ptr, size)
                }
            }
        }
    }
}
