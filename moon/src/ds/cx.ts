/** Junta as classes, ignorando as vazias. */
export function cx(...classes: Array<string | false | undefined>): string {
  return classes.filter(Boolean).join(" ");
}
