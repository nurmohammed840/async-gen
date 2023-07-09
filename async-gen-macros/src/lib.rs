use proc_macro::*;

#[proc_macro]
pub fn gen_inner(input: TokenStream) -> TokenStream {
    let mut tokens = input.into_iter();

    let Some(TokenTree::Group(crate_path)) = tokens.next() else { unimplemented!() };
    let crate_path = crate_path.stream();

    let mut has_yielded = false;
    let output = out(tokens, &mut has_yielded);

    let mut o = TokenStream::new();
    o.extend(crate_path.clone());
    o.push_colon2();
    o.push_ident("gen");

    o.push_group(Delimiter::Parenthesis, |o| {
        o.push_punct('|');
        o.push_ident("mut");
        o.push_ident("yield_");

        if !has_yielded {
            o.push_punct(':');
            o.extend(crate_path);
            o.push_colon2();
            o.push_ident("Yield");
        }

        o.push_punct('|');
        o.push_ident("async");
        o.push_ident("move");
        o.push_group(Delimiter::Brace, |o| {
            o.push_ident("let");
            o.push_ident("v");
            o.push_punct('=');
            o.push_ident("async");

            o.push(output);

            o.push_punct('.');
            o.push_ident("await");
            o.push_punct(';');
            o.push_ident("yield_");
            o.push_punct('.');
            o.push_ident("return_");
            o.push_group(Delimiter::Parenthesis, |o| {
                o.push_ident("v");
            });
        });
    });
    o
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
                        _ => expr.push(tt),
                    }
                }
                o.push_ident("yield_");
                o.push_punct('.');
                o.push_ident("yield_");
                o.push(Group::new(Delimiter::Parenthesis, expr));
                o.push_punct('.');
                o.push_ident("await");
                o.push_punct(';');
            }
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                o.push(out(g.stream().into_iter(), has_yielded));
            }
            _ => o.push(tt),
        }
    }
    Group::new(Delimiter::Brace, o)
}

trait TokenStreamExt {
    fn push<U>(&mut self, token: U)
    where
        U: Into<TokenTree>;

    #[inline]
    fn push_punct(&mut self, ch: char) {
        self.push(Punct::new(ch, Spacing::Alone))
    }

    #[inline]
    fn push_ident(&mut self, name: &str) {
        self.push(Ident::new(name, Span::call_site()))
    }

    #[inline]
    fn push_group(&mut self, delimiter: Delimiter, f: impl FnOnce(&mut TokenStream)) {
        let mut stream = TokenStream::new();
        f(&mut stream);
        self.push(Group::new(delimiter, stream))
    }

    #[inline]
    fn push_colon2(&mut self) {
        self.push(Punct::new(':', Spacing::Joint));
        self.push(Punct::new(':', Spacing::Alone));
    }
}

impl TokenStreamExt for TokenStream {
    #[inline]
    fn push<U>(&mut self, token: U)
    where
        U: Into<TokenTree>,
    {
        self.extend(std::iter::once(token.into()));
    }
}
