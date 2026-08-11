import { globSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const EXPORT_RE =
  /^export\s+(?:async\s+)?(?:function|const|let|class|interface|type|enum)\s+([A-Za-z_$][\w$]*)/gm;

/**
 * Escape a symbol for use inside a RegExp, since `$` in identifiers is an
 * anchor otherwise and would make `\b${symbol}\b` never match.
 */
function escapeForRegex(symbol) {
  return symbol.replace(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);
}

/**
 * @param {Map<string, string>} files path -> source text
 * @returns {Array<{file: string, symbol: string}>}
 */
export function findDeadExports(files) {
  const dead = [];
  for (const [file, text] of files) {
    for (const match of text.matchAll(EXPORT_RE)) {
      const symbol = match[1];
      // Identifier-character boundaries, not \b: \b treats `$` as a non-word
      // char, so a `$`-prefixed export would always read as dead.
      const word = new RegExp(String.raw`(?<![\w$])${escapeForRegex(symbol)}(?![\w$])`);
      const referenced = [...files].some(
        ([other, otherText]) => other !== file && word.test(otherText),
      );
      if (!referenced) dead.push({ file, symbol });
    }
  }
  return dead;
}

export function readSourceFiles(root, { includeTests }) {
  const listed = globSync('**/*.{ts,tsx}', { cwd: root });
  const wanted = includeTests
    ? listed
    : listed.filter((f) => !/\.(test|spec)\./.test(f) && !f.startsWith('test/'));
  return new Map(wanted.map((f) => [f, readFileSync(resolve(root, f), 'utf8')]));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  // --prod-only ignores tests, surfacing exports kept alive only by their own test file.
  const includeTests = !process.argv.includes('--prod-only');
  const files = readSourceFiles(resolve(import.meta.dirname, '../src'), { includeTests });
  for (const { file, symbol } of findDeadExports(files)) {
    console.log(`${file}\t${symbol}`);
  }
}
