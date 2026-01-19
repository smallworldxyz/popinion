
/**
 * Simple Markdown Renderer
 * Converts basic markdown syntax to HTML
 */
export const renderMarkdown = (text: string): string => {
    if (!text) return '';

    let html = text
        // Header 3
        .replace(/^### (.*$)/gim, '<h3>$1</h3>')
        // Header 4
        .replace(/^#### (.*$)/gim, '<h4>$1</h4>')
        // Bold
        .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
        // Italic
        .replace(/_(.*?)_/g, '<em>$1</em>')
        // Links
        .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank">$1</a>')
        // Lists
        .replace(/^\s*-\s+(.*$)/gim, '<li>$1</li>')
        .replace(/^\s*\d+\.\s+(.*$)/gim, '<li>$1</li>')
        // Quotes (simple)
        .replace(/^> (.*$)/gim, '<blockquote>$1</blockquote>')
        // Line breaks
        .replace(/\n/g, '<br>');

    // Wrap lists
    // This is a naive implementation; for nested lists or complex blocks, integration with a library is recommended.
    // But for simple tool output, this might suffice.

    return html;
};

export const truncateText = (text: string, length: number) => {
    if (!text || text.length <= length) return text;
    return text.substring(0, length) + '...';
};
