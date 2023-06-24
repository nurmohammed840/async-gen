use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    visit_mut::VisitMut,
    Block, Expr, Result, Stmt,
};

struct CodeBlock {
    crate_path: TokenStream2,
    stmts: Vec<Stmt>,
}

impl Parse for CodeBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let crate_path = match input.parse()? {
            TokenTree::Group(group) => group.stream(),
            _ => panic!(),
        };
        Ok(CodeBlock {
            crate_path,
            stmts: input.call(Block::parse_within)?,
        })
    }
}

struct EditCodeBlock {
    has_yielded: bool,
    unit: Box<syn::Expr>,
}

impl VisitMut for EditCodeBlock {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            syn::Expr::Yield(yield_expr) => {
                self.has_yielded = true;
                // syn::visit_mut::visit_expr_yield_mut(self, yield_expr);
                let value_expr = yield_expr.expr.as_ref().unwrap_or(&self.unit);
                *i = syn::parse_quote! { yield_.yield_(#value_expr).await };
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
        mut stmts,
    } = syn::parse_macro_input!(input);

    let mut edit = EditCodeBlock {
        has_yielded: false,
        unit: syn::parse_quote!(()),
    };
    for stmt in &mut stmts {
        edit.visit_stmt_mut(stmt);
    }
    let _ty = (!edit.has_yielded).then_some(quote! { ::<_, (), _> });
    TokenStream::from(quote! {
        #crate_path::gen #_ty (|mut yield_| async {
            let v = async { #(#stmts)* }.await;
            yield_.return_(v)
        })
    })
}
