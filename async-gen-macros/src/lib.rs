use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use syn::{
    parse::{Parse, ParseStream},
    visit_mut::VisitMut,
    Expr, ExprAsync, Result,
};

struct CodeBlock {
    crate_path: TokenStream2,
    async_block: ExprAsync,
}

impl Parse for CodeBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let crate_path = match input.parse()? {
            TokenTree::Group(group) => group.stream(),
            _ => panic!(),
        };
        Ok(CodeBlock {
            crate_path,
            async_block: input.parse()?,
        })
    }
}

#[derive(Default)]
struct EditCodeBlock {
    has_yielded: bool,
}

impl VisitMut for EditCodeBlock {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            syn::Expr::Yield(yield_expr) => {
                self.has_yielded = true;
                // syn::visit_mut::visit_expr_yield_mut(self, yield_expr);
                let value_expr = &yield_expr.expr;
                *i = syn::parse_quote! { __yield.yield_(#value_expr).await };
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
        }
    }

    // fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
    //     let mac_ident = mac.path.segments.last().map(|p| &p.ident);
    //     if mac_ident.map_or(false, |i| i == "gen") {
    //         return;
    //     }
    //     let out = &mut mac.tokens;
    //     let tokens = std::mem::replace(out, TokenStream2::new());

    //     let mut tts = tokens.into_iter();
    //     while let Some(tt) = tts.next() {
    //         match tt {
    //             TokenTree::Ident(i) if i == "yield" => {
    //                 // syn::parse2(tokens);
    //             }
    //             other => out.append(other),
    //         }
    //     }
    // }

    fn visit_item_mut(&mut self, _i: &mut syn::Item) {
        // match i {
        //     i => syn::visit_mut::visit_item_mut(self, i),
        // }
    }
}

#[proc_macro]
#[doc(hidden)]
pub fn gen_inner(input: TokenStream) -> TokenStream {
    let CodeBlock {
        crate_path,
        mut async_block,
    } = syn::parse_macro_input!(input);

    let mut edit = EditCodeBlock::default();
    edit.visit_block_mut(&mut async_block.block);

    let ret_ty = (!edit.has_yielded).then_some(quote::quote! { ::<(), _> });
    let async_token = async_block.async_token;
    let move_token = async_block.capture;
    let async_block = async_block.block;

    let tokens = quote::quote! {
        #crate_path::AsyncGen #ret_ty ::new(|mut __yield| async #move_token {
            let __body = #async_token #async_block  .await;
            return (__yield, __body);
        })
    };
    TokenStream::from(tokens)
}
