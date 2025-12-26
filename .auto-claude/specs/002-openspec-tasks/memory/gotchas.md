# Gotchas & Pitfalls

Things to watch out for in this codebase.

## [2025-12-26 16:26]
Verification command expected 11 but actual count is 10. The command `ls -la | grep -v archive | wc -l` counts 13 because it includes 'total', '.', and '..' lines. Use `ls | grep -v archive | wc -l` for accurate directory count (returns 10).

_Context: subtask-1-1 verification for openspec changes catalog_
