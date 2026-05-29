export type SnippetLang = 'html' | 'json' | 'url' | 'js';

export interface Token {
  text: string;
  cls?: 'tag' | 'attr' | 'string' | 'key' | 'keyword';
}

export function tokenize(code: string, lang: SnippetLang | undefined): Token[] {
  switch (lang) {
    case 'html':
      return tokenizeHtml(code);
    case 'json':
      return tokenizeJson(code);
    case 'js':
      return tokenizeJs(code);
    default:
      return [{ text: code }];
  }
}

interface TokenSpec {
  regex: RegExp;
  cls?: Token['cls'];
  // Optional sub-pattern emitter for matches with internal structure (e.g.
  // `attr="value"` needs two tokens of different classes).
  emit?: (match: RegExpExecArray) => Token[];
}

function applySpecs(code: string, specs: TokenSpec[]): Token[] {
  // Greedy left-to-right scan: at each position, find the earliest match
  // among the spec regexes; emit any unmatched prefix as a plain text token,
  // then emit the matched tokens; continue from after the match.
  const tokens: Token[] = [];
  let i = 0;
  while (i < code.length) {
    let earliest: { start: number; spec: TokenSpec; match: RegExpExecArray } | null = null;
    for (const spec of specs) {
      spec.regex.lastIndex = i;
      const m = spec.regex.exec(code);
      if (m && (earliest === null || m.index < earliest.start)) {
        earliest = { start: m.index, spec, match: m };
      }
    }
    if (!earliest) {
      tokens.push({ text: code.slice(i) });
      break;
    }
    if (earliest.start > i) {
      tokens.push({ text: code.slice(i, earliest.start) });
    }
    if (earliest.spec.emit) {
      tokens.push(...earliest.spec.emit(earliest.match));
    } else {
      tokens.push({ text: earliest.match[0], cls: earliest.spec.cls });
    }
    i = earliest.start + earliest.match[0].length;
  }
  return tokens;
}

function tokenizeHtml(code: string): Token[] {
  return applySpecs(code, [
    // attr="value"
    {
      regex: /([a-zA-Z][a-zA-Z0-9-]*)=("[^"]*")/g,
      emit: (m) => [{ text: m[1], cls: 'attr' }, { text: '=' }, { text: m[2], cls: 'string' }],
    },
    // <tag and </tag (without the > yet — preserves attrs for next pass)
    {
      regex: /<\/?[a-zA-Z][a-zA-Z0-9-]*/g,
      cls: 'tag',
    },
  ]);
}

function tokenizeJson(code: string): Token[] {
  return applySpecs(code, [
    // "key":
    {
      regex: /("[^"]*")(\s*:)/g,
      emit: (m) => [{ text: m[1], cls: 'key' }, { text: m[2] }],
    },
    // any "string"
    { regex: /"[^"]*"/g, cls: 'string' },
  ]);
}

function tokenizeJs(code: string): Token[] {
  return applySpecs(code, [
    { regex: /\b(?:var|const|let|function|return|true|false|null)\b/g, cls: 'keyword' },
    { regex: /"[^"]*"/g, cls: 'string' },
  ]);
}
