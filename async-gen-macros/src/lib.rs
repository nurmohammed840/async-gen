use proc_macro2::{
    token_stream, Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree,
};
use quote::{quote, TokenStreamExt};

#[proc_macro]
pub fn gen_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut tokens = TokenStream::from(input).into_iter();
    let crate_path = match tokens.next().unwrap() {
        TokenTree::Group(group) => group.stream(),
        _ => todo!(),
    };

    let mut has_yielded = false;
    let output = out(tokens, &mut has_yielded);
    let _ty = (!has_yielded).then_some(quote! { ::<_, (), _> });

    // quote! {
    //     #crate_path::gen #_ty (|mut yield_| async {
    //         let v = async #output.await;
    //         yield_.return_(v)
    //     })
    // }
    // .into()
    {
        let mut s = TokenStream::new();

        quote::ToTokens::to_tokens(&crate_path, &mut s);

        s.append(Punct::new(':', Spacing::Joint));
        s.append(Punct::new(':', Spacing::Alone));
        s.append(z::ident("gen"));

        quote::ToTokens::to_tokens(&_ty, &mut s);

        s.append({
            let mut s = TokenStream::new();
            s.append(z::punct('|'));
            s.append(z::ident("mut"));
            s.append(z::ident("yield_"));
            s.append(z::punct('|'));
            s.append(z::ident("async"));
            s.append({
                let mut s = TokenStream::new();
                s.append(z::ident("let"));
                s.append(z::ident("v"));
                s.append(z::punct('='));
                s.append(z::ident("async"));

                quote::ToTokens::to_tokens(&output, &mut s);

                s.append(z::punct('.'));
                s.append(z::ident("await"));
                s.append(z::punct(';'));
                s.append(z::ident("yield_"));
                s.append(z::punct('.'));
                s.append(z::ident("return_"));
                s.append({
                    let mut s = TokenStream::new();
                    s.append(z::ident("v"));
                    Group::new(Delimiter::Parenthesis, s)
                });
                Group::new(Delimiter::Brace, s)
            });
            Group::new(Delimiter::Parenthesis, s)
        });
        s
    }
    .into()
}

fn out(mut tokens: token_stream::IntoIter, has_yielded: &mut bool) -> Group {
    let mut o = TokenStream::new();
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
                o.append(z::ident("yield_"));
                o.append(z::punct('.'));
                o.append(z::ident("yield_"));
                o.append(Group::new(Delimiter::Parenthesis, expr));
                o.append(z::punct('.'));
                o.append(z::ident("await"));
                o.append(z::punct(';'));
            }
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                o.append(out(g.stream().into_iter(), has_yielded));
            }
            _ => o.append(tt),
        }
    }
    Group::new(Delimiter::Brace, o)
}

mod z {
    use super::*;
    pub fn punct(ch: char) -> Punct {
        Punct::new(ch, Spacing::Alone)
    }
    pub fn ident(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }
}
