use mlc_syntax::{
    lexer::Lexed,
    parser::out::{TempGlobalStmt, TempModule},
};

// Reorder complete import declarations, carrying their comments with them.
// Other declarations are barriers: in particular, never move imports across using.
pub(super) fn sorted(source: &str, lexed: &Lexed, module: &TempModule) -> Option<String> {
    let mut edits = Vec::new();
    let mut start = 0;
    while start < module.len() {
        if !matches!(module[start].0, TempGlobalStmt::Import(_)) {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < module.len() && matches!(module[end].0, TempGlobalStmt::Import(_)) {
            end += 1;
        }
        let mut order: Vec<_> = (start..end).collect();
        order.sort_by_key(|&index| {
            let TempGlobalStmt::Import(import) = &module[index].0 else {
                unreachable!()
            };
            let path = import.path();
            (
                path.len(),
                path.iter().map(|part| part.chars().count()).sum::<usize>()
                    + path.len().saturating_sub(1) * 2,
            )
        });
        if order.iter().copied().ne(start..end) {
            let mut boundaries = vec![module[start].1.start];
            for index in start..end {
                let span = module[index].1;
                let limit = module
                    .get(index + 1)
                    .map_or(source.len(), |(_, span)| span.start);
                let mut trailing = span.end;
                for comment in lexed
                    .comments
                    .iter()
                    .filter(|comment| comment.span.start >= span.end && comment.span.start < limit)
                {
                    if source[trailing..comment.span.start].contains(['\r', '\n']) {
                        break;
                    }
                    trailing = comment.span.end;
                }
                boundaries.push(trailing);
            }
            let mut replacement = String::new();
            for index in order {
                let offset = index - start;
                replacement.push_str(source[boundaries[offset]..boundaries[offset + 1]].trim());
                replacement.push('\n');
            }
            edits.push((boundaries[0]..*boundaries.last().unwrap(), replacement));
        }
        start = end;
    }
    if edits.is_empty() {
        return None;
    }
    let mut result = source.to_owned();
    for (span, replacement) in edits.into_iter().rev() {
        result.replace_range(span, &replacement);
    }
    Some(result)
}
