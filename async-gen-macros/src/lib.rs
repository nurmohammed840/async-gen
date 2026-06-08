mod tokens;
use proc_macro::*;
use tokens::*;

#[proc_macro]
pub fn gen_inner(input: TokenStream) -> TokenStream {
    let mut tokens = input.into_iter();

    let Some(TokenTree::Group(crate_path)) = tokens.next() else {
        unimplemented!()
    };
    let crate_path = crate_path.stream();

    let mut has_yielded = false;
    let transformed_body = transform_yield_kw(tokens, &mut has_yielded);

    let mut o = TokenStream::new();

    // #crate_path ::gen(|mut _c| async move #out )
    o.extend(crate_path.clone());
    o.extend([
        punct_join(':'),
        punct(':'),
        ident("gen"),
        group('(', |o| {
            o.extend([punct('|'), ident("mut"), ident("_c")]);

            if !has_yielded {
                o.push(punct(':'));
                o.extend(crate_path);
                o.extend([punct_join(':'), punct(':'), ident("Yielder")]);
            }

            o.extend([
                punct('|'),
                ident("async"),
                ident("move"),
                group('{', |o| {
                    o.extend([
                        ident("let"),
                        ident("v"),
                        punct('='),
                        ident("async"),
                        transformed_body,
                        punct('.'),
                        ident("await"),
                        punct(';'),
                        // --------------
                        ident("_c"),
                        punct('.'),
                        ident("return_"),
                        group('(', |o| {
                            o.push(ident("v"));
                        }),
                    ])
                }),
            ]);
        }),
    ]);
    o
}

fn transform_yield_kw(mut tokens: token_stream::IntoIter, has_yielded: &mut bool) -> TokenTree {
    group('{', |o| {
        while let Some(tt) = tokens.next() {
            match tt {
                TokenTree::Ident(name) if name.to_string() == "yield" => {
                    *has_yielded = true;
                    let expr = group('(', |o| {
                        for tt in &mut tokens {
                            match tt {
                                TokenTree::Punct(p) if p.as_char() == ';' => break,
                                _ => o.push(tt),
                            }
                        }
                        if o.is_empty() {
                            o.push(group('(', |_| {}));
                        };
                    });
                    o.extend([
                        ident("_c"),
                        punct('.'),
                        ident("yield_"),
                        expr,
                        punct('.'),
                        ident("await"),
                        punct(';'),
                    ]);
                }
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                    o.push(transform_yield_kw(g.stream().into_iter(), has_yielded));
                }
                _ => o.push(tt),
            }
        }
    })
}

trait TokenStreamExt {
    fn push(&mut self, tt: TokenTree);
}

impl TokenStreamExt for TokenStream {
    #[inline]
    fn push(&mut self, tt: TokenTree) {
        self.extend(Some(tt));
    }
}
