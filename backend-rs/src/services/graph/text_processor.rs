//! Text chunking (port of text_processor.py / split_text_into_chunks).

const SEPARATORS: [&str; 7] = [". ", "! ", "? ", "\n\n", ".\n", "!\n", "?\n"];

/// Split text into chunks of ~`chunk_size` characters with `overlap` chars of
/// overlap, preferring to break at sentence boundaries. Operates on chars, not
/// bytes, so multibyte text (CJK etc.) never splits mid-codepoint.
pub fn split_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let chunk_size = chunk_size.max(1);
    let chars: Vec<char> = text.chars().collect();

    if chars.len() <= chunk_size {
        return if text.trim().is_empty() { vec![] } else { vec![text.to_string()] };
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < chars.len() {
        let mut end = (start + chunk_size).min(chars.len());

        // Try to split at a sentence boundary within the window.
        if end < chars.len() {
            let window: String = chars[start..end].iter().collect();
            for sep in SEPARATORS {
                if let Some(byte_pos) = window.rfind(sep) {
                    let char_pos = window[..byte_pos].chars().count();
                    if char_pos > chunk_size * 3 / 10 {
                        end = start + char_pos + sep.chars().count();
                        break;
                    }
                }
            }
        }

        let chunk: String = chars[start..end].iter().collect();
        let chunk = chunk.trim().to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }

        if end >= chars.len() {
            break;
        }
        // Guard against overlap >= progress causing an infinite loop.
        start = end.saturating_sub(overlap).max(start + 1);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_single_chunk() {
        assert_eq!(split_text("hello world", 500, 50), vec!["hello world"]);
    }

    #[test]
    fn blank_text_yields_no_chunks() {
        assert!(split_text("   \n\n  ", 500, 50).is_empty());
    }

    #[test]
    fn splits_at_sentence_boundary() {
        let text = "First sentence here. Second sentence is a bit longer. Third one.";
        let chunks = split_text(text, 30, 5);
        assert!(chunks.len() >= 2);
        assert!(chunks[0].ends_with('.'), "chunk should end at sentence: {:?}", chunks[0]);
    }

    #[test]
    fn overlap_repeats_content() {
        let text = "a".repeat(100) + ". " + &"b".repeat(100);
        let chunks = split_text(&text, 80, 20);
        assert!(chunks.len() >= 2);
        // Every char of the input appears in some chunk.
        let joined: String = chunks.concat();
        assert!(joined.contains(&"a".repeat(80)));
        assert!(joined.contains(&"b".repeat(80)));
    }

    #[test]
    fn no_infinite_loop_when_overlap_exceeds_chunk() {
        let text = "x".repeat(1000);
        let chunks = split_text(&text, 10, 100);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn multibyte_text_does_not_panic() {
        let text = "这是一个测试。".repeat(200);
        let chunks = split_text(&text, 100, 10);
        assert!(chunks.len() > 1);
        for c in &chunks {
            assert!(c.chars().count() <= 110);
        }
    }
}
