// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

export interface Locator {
  kind: "name" | "lexical";
  value: string;
}
export interface Selector {
  kind: "selector";
  selector: "e" | "tag" | "schema" | "r" | "object" | "ref";
  locators: Locator[];
}
export type Atom =
  { kind: "text"; text: string } | { kind: "concept"; text: string } | Selector;
export type Argument = string | number;
export interface Directive {
  name: string;
  positional: Argument[];
  named: Record<string, Argument>;
}
interface Preference {
  negative: boolean;
  operand: Atom | { kind: "key"; value: string };
}
export interface QuerySyntax {
  source: string;
  intentText: string;
  cues: Atom[];
  directives: Directive[];
  preferences: Preference[];
}
