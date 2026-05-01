use std::collections::VecDeque;

const MIN_VISIBLE: usize = 1;

fn calc_visible_impl<F>(
    messages: &[String],
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
    get_lines: F,
) -> (usize, usize)
where
    F: Fn(&str) -> usize,
{
    let total_items = messages.len();
    if total_items == 0 {
        return (0, 0);
    }

    if auto_scroll {
        let (visible, _offset) = calc_auto_scroll(messages, usable_height, get_lines);
        (visible, total_items.saturating_sub(visible))
    } else {
        let visible = calc_manual_scroll(messages, scroll_offset, usable_height, get_lines);
        (visible, scroll_offset)
    }
}

fn calc_auto_scroll<F>(messages: &[String], usable_height: usize, get_lines: F) -> (usize, usize)
where
    F: Fn(&str) -> usize,
{
    let mut used = 0;
    let mut count = 0;
    for msg in messages.iter().rev() {
        let msg_lines = get_lines(msg);
        if used > 0 && used + msg_lines > usable_height {
            break;
        }
        used += msg_lines;
        count += 1;
    }
    (count, 0)
}

fn calc_manual_scroll<F>(
    messages: &[String],
    scroll_offset: usize,
    usable_height: usize,
    get_lines: F,
) -> usize
where
    F: Fn(&str) -> usize,
{
    if scroll_offset >= messages.len() {
        return MIN_VISIBLE;
    }
    let mut used = 0;
    let mut count = 0;
    for msg in messages.iter().skip(scroll_offset) {
        let msg_lines = get_lines(msg);
        if used > 0 && used + msg_lines > usable_height {
            break;
        }
        used += msg_lines;
        count += 1;
    }
    count.max(MIN_VISIBLE)
}

/// Calculate visible items and effective offset for tuple-based messages with auto/manual scroll
pub fn calc_visible_tuples(
    messages: &VecDeque<(String, Option<String>)>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let msgs: Vec<String> = messages.iter().map(|(m, _)| m.clone()).collect();
    calc_visible_impl(
        &msgs,
        auto_scroll,
        scroll_offset,
        text_width,
        usable_height,
        |m| count_lines(m, text_width),
    )
}

/// Calculate visible items and effective offset for string-based messages with auto/manual scroll
pub fn calc_visible_strings(
    messages: &VecDeque<String>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let msgs: Vec<String> = messages.iter().cloned().collect();
    calc_visible_impl(
        &msgs,
        auto_scroll,
        scroll_offset,
        text_width,
        usable_height,
        |m| count_lines(m, text_width),
    )
}

/// Count wrapped lines of text, accounting for ANSI codes and terminal width
pub fn count_lines(text: &str, text_width: usize) -> usize {
    let clean_text = p2p_app::strip_ansi_codes(text);
    let lines: Vec<&str> = clean_text.split('\n').collect();

    if lines.is_empty() {
        return 1;
    }

    let mut total = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            if i < lines.len() - 1 {
                total += 1;
            }
        } else {
            total += line.len().div_ceil(text_width);
        }
    }
    total.max(1)
}
