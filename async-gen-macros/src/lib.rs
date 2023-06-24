use proc_macro::*;
use z::TokenStreamExt;

#[proc_macro]
pub fn gen_inner(input: TokenStream) -> TokenStream {
    let mut tokens = input.into_iter();
    let crate_path = match tokens.next().unwrap() {
        TokenTree::Group(group) => group.stream(),
        _ => unimplemented!(),
    };

    let mut has_yielded = false;
    let output = out(tokens, &mut has_yielded);
    {
        let mut s = TokenStream::new();

        s.extend(crate_path.clone());
        s.append_colon2();
        s.append_ident("gen");

        s.append({
            let mut s = TokenStream::new();
            s.append_punct('|');
            s.append_ident("mut");
            s.append_ident("yield_");

            if !has_yielded {
                s.append_punct(':');
                s.extend(crate_path);
                s.append_colon2();
                s.append_ident("Yield");
            }

            s.append_punct('|');
            s.append_ident("async");
            s.append({
                let mut s = TokenStream::new();
                s.append_ident("let");
                s.append_ident("v");
                s.append_punct('=');
                s.append_ident("async");

                s.append(output);

                s.append_punct('.');
                s.append_ident("await");
                s.append_punct(';');
                s.append_ident("yield_");
                s.append_punct('.');
                s.append_ident("return_");
                s.append({
                    let mut s = TokenStream::new();
                    s.append_ident("v");
                    Group::new(Delimiter::Parenthesis, s)
                });
                Group::new(Delimiter::Brace, s)
            });
            Group::new(Delimiter::Parenthesis, s)
        });
        s
    }
}

fn out(mut tokens: token_stream::IntoIter, has_yielded: &mut bool) -> Group {
    let mut o = TokenStream::new();

    while let Some(tt) = tokens.next() {
        match tt {
            TokenTree::Ident(name) if name.to_string() == "yield" => {
                *has_yielded = true;
                let mut expr = TokenStream::new();
                for tt in &mut tokens {
                    match tt {
                        TokenTree::Punct(p) if p.as_char() == ';' => break,
                        _ => expr.append(tt),
                    }
                }
                o.append_ident("yield_");
                o.append_punct('.');
                o.append_ident("yield_");
                o.append(Group::new(Delimiter::Parenthesis, expr));
                o.append_punct('.');
                o.append_ident("await");
                o.append_punct(';');
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

    pub trait TokenStreamExt {
        fn append<U>(&mut self, token: U)
        where
            U: Into<TokenTree>;

        #[inline]
        fn append_punct(&mut self, ch: char) {
            self.append(Punct::new(ch, Spacing::Alone))
        }

        #[inline]
        fn append_ident(&mut self, name: &str) {
            self.append(Ident::new(name, Span::call_site()))
        }

        #[inline]
        fn append_colon2(&mut self) {
            self.append(Punct::new(':', Spacing::Joint));
            self.append(Punct::new(':', Spacing::Alone));
        }
    }

    impl TokenStreamExt for TokenStream {
        #[inline]
        fn append<U>(&mut self, token: U)
        where
            U: Into<TokenTree>,
        {
            self.extend(std::iter::once(token.into()));
        }
    }
}
