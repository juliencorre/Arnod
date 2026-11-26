/**
 * Libellés d'affichage des assertions : opérateurs d'OpenCollection et de Bruno, et l'assertion de
 * durée telle qu'on la lit (« responseTime inférieur à 500 ms », 5.1-c).
 */
import type { Assertion, AssertionOperator } from "@/types/ipc";

/** Opérateurs d'OpenCollection et de Bruno (R4), dans l'ordre du menu. */
export const OPERATORS: { id: AssertionOperator; label: string }[] = [
  { id: "equals", label: "est égal à" },
  { id: "notEquals", label: "est différent de" },
  { id: "contains", label: "contient" },
  { id: "notContains", label: "ne contient pas" },
  { id: "startsWith", label: "commence par" },
  { id: "endsWith", label: "se termine par" },
  { id: "lessThan", label: "est inférieur à" },
  { id: "lessOrEqual", label: "est inférieur ou égal à" },
  { id: "greaterThan", label: "est supérieur à" },
  { id: "greaterOrEqual", label: "est supérieur ou égal à" },
  { id: "between", label: "est entre (min, max)" },
  { id: "in", label: "est parmi (a, b, …)" },
  { id: "notIn", label: "n'est pas parmi (a, b, …)" },
  { id: "length", label: "a pour longueur" },
  { id: "exists", label: "existe" },
  { id: "isUndefined", label: "n'existe pas" },
  { id: "isNull", label: "est null" },
  { id: "isEmpty", label: "est vide" },
  { id: "isNotEmpty", label: "n'est pas vide" },
  { id: "isTruthy", label: "est vrai (truthy)" },
  { id: "isFalsy", label: "est faux (falsy)" },
  { id: "isNumber", label: "est un nombre" },
  { id: "isText", label: "est un texte" },
  { id: "isBoolean", label: "est un booléen" },
  { id: "isArray", label: "est une liste" },
  { id: "isJson", label: "est un objet JSON" },
  { id: "matches", label: "correspond à l'expression (non évaluée)" },
  { id: "notMatches", label: "ne correspond pas à l'expression (non évaluée)" },
];

/** « responseTime inférieur à 500 ms » : l'assertion de durée, sans le « est » du menu (5.1-c). */
export function durationLabel(assertion: Assertion): string {
  const label = OPERATORS.find((o) => o.id === assertion.operator)?.label ?? assertion.operator;
  const value = assertion.value ?? "";
  return `${assertion.target} ${label.replace(/^est /, "")} ${String(value)} ms`.trim();
}
