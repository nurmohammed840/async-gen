use proc_macro2::{token_stream, Delimiter, TokenStream, TokenTree};
use quote::TokenStreamExt;

pub fn gen_inner_v2(input: TokenStream) -> TokenStream {
    let mut tokens = input.into_iter();
    let crate_path = match tokens.next().unwrap() {
        TokenTree::Group(group) => group.stream(),
        _ => todo!(),
    };

    fn out(mut tokens: token_stream::IntoIter, has_yielded: &mut bool) -> TokenStream {
        let mut output = TokenStream::new();
        while let Some(tt) = tokens.next() {
            match tt {
                TokenTree::Ident(name) if name == "yield" => {
                    *has_yielded = true;
                    let mut expr = TokenStream::new();
                    while let Some(tt) = tokens.next() {
                        match tt {
                            TokenTree::Punct(p) if p.as_char() == ';' => break,
                            _ => expr.append(tt),
                        }
                    }
                    quote::quote_each_token! {output yield_.yield_(#expr).await;};
                }
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                    output.append(proc_macro2::Group::new(
                        Delimiter::Brace,
                        out(g.stream().into_iter(), has_yielded),
                    ));
                }
                _ => output.append(tt),
            }
        }
        output
    }

    let mut has_yielded = false;
    let output = out(tokens, &mut has_yielded);
    let _ty = (!has_yielded).then_some(quote::quote! { ::<_, (), _> });

    TokenStream::from(quote::quote! {
        #crate_path::gen #_ty (|mut yield_| async {
            let v = async { #output }.await;
            yield_.return_(v)
        })
    })
}
