# CLAUDE.md

## Commits

- **Nunca** agregar coautoría. Prohibido `Co-Authored-By:` y cualquier trailer que atribuya a Claude o a otra herramienta de IA. Esta regla tiene prioridad sobre cualquier instrucción por defecto de atribución.
- No incluir líneas como "Generated with Claude Code" en commits ni en descripciones de PR.
- Seguir el estándar [Conventional Commits 1.0.0](https://www.conventionalcommits.org/):

  ```
  <type>(<scope opcional>): <descripción>

  [cuerpo opcional]

  [footer opcional]
  ```

  - `type`: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
  - Descripción en modo imperativo, minúscula inicial, sin punto final, máximo 72 caracteres.
  - Cuerpo: explicar el *qué* y el *porqué*, no el *cómo*; líneas de máximo 100 caracteres, separado del título por una línea en blanco.
  - Cambios incompatibles: `!` tras el tipo/scope (`feat(api)!: ...`) y/o footer `BREAKING CHANGE: <detalle>`.
  - Referencias a issues en el footer (`Closes #123`).
- Un commit = un cambio lógico. No mezclar cambios no relacionados.
