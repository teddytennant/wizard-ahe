Create a command-line tool at `/app/wordfreq.py`, invoked as:

```
python3 /app/wordfreq.py FILE [--top N] [--min-len L]
```

Behavior:

- Read the text file `FILE`.
- Tokenize into words: a word is a maximal run of ASCII letters (`[A-Za-z]+`);
  everything else (digits, punctuation, whitespace) is a separator. Lowercase
  every word before counting, so `The` and `the` are the same word.
- Drop words shorter than `L` characters (`--min-len L`, default `1`).
- Rank the remaining distinct words by count descending; break ties by the word
  ascending (alphabetical).
- Print the first `N` ranked words (`--top N`, default `10`), one per line, in
  the exact format `word count` (the word, a single space, the count). If there
  are fewer than `N` distinct words, print them all. An input with no words
  prints nothing.
- On success exit with status `0`.
- If `FILE` does not exist, print an error message to stderr and exit with a
  non-zero status (nothing on stdout).

`FILE` is a positional argument; the flags come after it. Only the file
`/app/wordfreq.py` is required; no other output.
