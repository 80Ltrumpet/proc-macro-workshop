use proc_macro2::{Group, Literal, TokenStream, TokenTree};
use quote::{format_ident, TokenStreamExt};
use syn::{
    braced,
    buffer::TokenBuffer,
    parse::{Parse, ParseStream, Result},
    parse_macro_input, Ident, LitInt, Token,
};

struct Seq {
    param: Ident,
    start: usize,
    end: usize,
    token_stream: TokenStream,
}

impl Parse for Seq {
    fn parse(input: ParseStream) -> Result<Self> {
        let param = input.parse()?;
        input.parse::<Token![in]>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Token![..]>()?;
        let end = input.parse::<LitInt>()?.base10_parse()?;
        let block;
        braced!(block in input);
        let token_stream = block.parse()?;
        Ok(Self {
            param,
            start,
            end,
            token_stream,
        })
    }
}

fn replace_param(tokens: TokenStream, param: &Ident, value: usize) -> TokenStream {
    tokens
        .into_iter()
        .map(|tree| match tree {
            TokenTree::Ident(ident) if &ident == param => {
                TokenTree::Literal(Literal::usize_unsuffixed(value))
            }
            TokenTree::Group(group) => {
                let delimiter = group.delimiter();
                TokenTree::Group(Group::new(
                    delimiter,
                    replace_param(group.stream(), param, value),
                ))
            }
            other => other,
        })
        .collect()
}

fn paste_ident(tokens: TokenStream) -> TokenStream {
    let mut out_tokens = TokenStream::new();
    let buffer = TokenBuffer::new2(tokens);
    let mut cursor = buffer.begin();
    while !cursor.eof() {
        if let Some((prefix, next)) = cursor.clone().ident() {
            if let Some((tilde, next)) = next.punct() {
                if tilde.as_char() == '~' {
                    // This isn't exactly what I want, because it will "paste" for all occurences
                    // of `~` instead of only those concatenating the parameter.
                    if let Some((lit, next)) = next.clone().literal() {
                        out_tokens.append(format_ident!("{prefix}{lit}"));
                        cursor = next;
                    } else {
                        out_tokens.append(prefix);
                        out_tokens.append(tilde);
                        cursor = next;
                    }
                } else {
                    out_tokens.append(prefix);
                    out_tokens.append(tilde);
                    cursor = next;
                }
            } else {
                out_tokens.append(prefix);
                cursor = next;
            }
        } else if let Some((TokenTree::Group(group), next)) = cursor.clone().token_tree() {
            let delimiter = group.delimiter();
            out_tokens.append(TokenTree::Group(Group::new(delimiter, paste_ident(group.stream()))));
            cursor = next;
        } else {
            let (tt, next) = cursor.token_tree().unwrap();
            out_tokens.append(tt);
            cursor = next;
        }
    }
    out_tokens
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Seq {
        param,
        start,
        end,
        token_stream,
    } = parse_macro_input!(input as Seq);

    let mut expanded = proc_macro2::TokenStream::new();
    for i in start..end {
        expanded.extend(replace_param(token_stream.clone(), &param, i));
    }
    expanded = paste_ident(expanded);

    expanded.into()
}
