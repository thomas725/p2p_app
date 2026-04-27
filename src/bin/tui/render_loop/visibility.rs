use std::collections::VecDeque;

/// Calculate visible items and effective offset for tuple-based messages with auto/manual scroll
pub fn calc_visible_tuples(
    messages: &VecDeque<(String, Option<String>)>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let total_items = messages.len();
    if total_items == 0 {
        return (0, 0);
    }

    if auto_scroll {
        let mut used = 0;
        let mut count = 0;
        for (msg, _) in messages.iter().rev() {
            let msg_lines = count_lines(msg, text_width);
            if used + msg_lines > usable_height {
                break;
            }
            used += msg_lines;
            count += 1;
        }
        let visible = count;
        let offset = total_items.saturating_sub(visible);
        (visible, offset)
    } else {
        let visible = if scroll_offset < total_items {
            let mut used = 0;
            let mut count = 0;
            for (msg, _) in messages.iter().skip(scroll_offset) {
                let msg_lines = count_lines(msg, text_width);
                if used > 0 && used + msg_lines > usable_height {
                    break;
                }
                used += msg_lines;
                count += 1;
            }
            count.max(1)
        } else {
            1
        };
        (visible, scroll_offset)
    }
}

/// Calculate visible items and effective offset for string-based messages with auto/manual scroll
pub fn calc_visible_strings(
    messages: &VecDeque<String>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let total_items = messages.len();
    if total_items == 0 {
        return (0, 0);
    }

    if auto_scroll {
        let mut used = 0;
        let mut count = 0;
        for msg in messages.iter().rev() {
            let msg_lines = count_lines(msg, text_width);
            if used > 0 && used + msg_lines > usable_height {
                break;
            }
            used += msg_lines;
            count += 1;
        }
        let visible = count;
        let offset = total_items.saturating_sub(visible);
        (visible, offset)
    } else {
        let visible = if scroll_offset < total_items {
            let mut used = 0;
            let mut count = 0;
            for msg in messages.iter().skip(scroll_offset) {
                let msg_lines = count_lines(msg, text_width);
                if used > 0 && used + msg_lines > usable_height {
                    break;
                }
                used += msg_lines;
                count += 1;
            }
            count.max(1)
        } else {
            1
        };
        (visible, scroll_offset)
    }
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
