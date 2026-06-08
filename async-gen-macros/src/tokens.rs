use proc_macro::*;

pub fn punct_join(ch: char) -> TokenTree {
    Punct::new(ch, Spacing::Joint).into()
}

pub fn punct(ch: char) -> TokenTree {
    Punct::new(ch, Spacing::Alone).into()
}

pub fn ident(name: &str) -> TokenTree {
    Ident::new(name, Span::call_site()).into()
}

pub fn group(ch: char, f: impl FnOnce(&mut TokenStream)) -> TokenTree {
    let mut stream = TokenStream::new();
    f(&mut stream);
    Group::new(
        match ch {
            '{' => Delimiter::Brace,
            '[' => Delimiter::Bracket,
            '(' => Delimiter::Parenthesis,
            _ => Delimiter::None,
        },
        stream,
    )
    .into()
}
