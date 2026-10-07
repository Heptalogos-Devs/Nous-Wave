// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { createToken, Lexer, type IToken } from "chevrotain";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  expression,
  type Atom,
  type Argument,
  type Directive,
  type Expression,
  type Locator,
  type Selector,
} from "./syntax.js";

const Space = createToken({
  name: "Space",
  pattern: /\s+/,
  group: Lexer.SKIPPED,
});
const Quoted = createToken({
  name: "Quoted",
  pattern: /"(?:[^"\\\r\n]|\\.)*"/,
});
const Lexical = createToken({
  name: "Lexical",
  pattern: /[a-z]+:[a-z]+(?:-[a-z]+){3}\b/,
});
const Duration = createToken({
  name: "Duration",
  pattern: /\d+(?:ms|s|m|h|d|w)\b/,
});
const NumberToken = createToken({ name: "Number", pattern: /\d+(?:\.\d+)?/ });
const Name = createToken({ name: "Name", pattern: /[A-Za-z_][A-Za-z0-9_]*/ });
const Symbols = Object.fromEntries(
  ["&&", "||", "(", ")", ",", "=", "@", "$", "#", "+", "-", "*"].map(
    (symbol, index) => [
      symbol,
      createToken({ name: `Symbol${index}`, pattern: symbol }),
    ],
  ),
);
const lexer = new Lexer([
  Space,
  Quoted,
  Lexical,
  Duration,
  NumberToken,
  Name,
  ...Object.values(Symbols),
]);

export const resultDomainOrder = [
  "memory",
  "schema",
  "episode",
  "journal",
  "evidence",
  "resource",
];

const selectorNames = new Set<Selector["selector"]>([
  "e",
  "tag",
  "schema",
  "r",
  "object",
  "ref",
]);

function isSelector(value: string): value is Selector["selector"] {
  return selectorNames.has(value as Selector["selector"]);
}

export function parse(source: string): Expression {
  if (Buffer.byteLength(source) > 32768)
    throw new ConnectError(
      "NousQL source exceeds 32 KiB",
      Code.ResourceExhausted,
    );
  const result = lexer.tokenize(source);
  if (result.errors.length)
    throw new ConnectError(
      `NousQL lexical error at ${result.errors[0]!.offset}`,
      Code.InvalidArgument,
    );
  if (result.tokens.length > 2048)
    throw new ConnectError(
      "NousQL token bound exceeded",
      Code.ResourceExhausted,
    );
  const parser = new Parser(result.tokens);
  const output = parser.query(0);
  if (parser.peek()) parser.fail("Unexpected token; use explicit && or ||");
  validateDirectives(output, true);
  return output;
}
function validateDirectives(node: Expression, root: boolean) {
  const seen = new Set<string>();
  for (const d of node.directives) {
    if (
      !root &&
      [
        "return",
        "asof",
        "history",
        "effort",
        "limit",
        "diagnostics",
        "explore",
        "materialize",
      ].includes(d.name)
    )
      throw new ConnectError(
        `$${d.name} is only allowed at query root`,
        Code.InvalidArgument,
      );
    const key = d.name === "time" ? `time:${d.positional[0]}` : d.name;
    if (seen.has(key))
      throw new ConnectError(
        `Duplicate directive $${key}`,
        Code.InvalidArgument,
      );
    seen.add(key);
    if (d.name === "return") {
      if (
        Object.keys(d.named).length ||
        !d.positional.length ||
        d.positional.some(
          (v) =>
            typeof v !== "string" ||
            !["cognition", ...resultDomainOrder].includes(v),
        ) ||
        new Set(d.positional).size !== d.positional.length ||
        (d.positional.includes("cognition") && d.positional.length !== 1)
      )
        throw new ConnectError(
          "Invalid result projection",
          Code.InvalidArgument,
        );
    }
  }
  for (const child of node.children) validateDirectives(child, false);
}
class Parser {
  private index = 0;
  private depth = 0;
  constructor(private readonly tokens: IToken[]) {}
  peek() {
    return this.tokens[this.index];
  }
  fail(message: string): never {
    throw new ConnectError(
      `${message} at ${this.peek()?.startOffset ?? "end"}`,
      Code.InvalidArgument,
    );
  }
  private take(image?: string) {
    const token = this.peek();
    if (!token || (image !== undefined && token.image !== image))
      this.fail(`Expected ${image ?? "token"}`);
    this.index++;
    return token;
  }
  query(minimum: number): Expression {
    if (++this.depth > 16) this.fail("NousQL nesting bound exceeded");
    let left: Expression;
    if (this.peek()?.image === "(") {
      this.take("(");
      left = this.query(0);
      this.take(")");
    } else left = expression(this.atom());
    while (true) {
      const symbol = this.peek()?.image;
      if (symbol === "$") {
        left.directives.push(this.directive());
        continue;
      }
      if (symbol === "+" || symbol === "-") {
        this.take();
        let parenthesized = false;
        if (this.peek()?.image === "(") {
          this.take();
          parenthesized = true;
        }
        const operand =
          this.peek()?.tokenType === Name
            ? { kind: "key" as const, value: this.take().image }
            : this.atom();
        if (operand.kind === "key") {
          if (operand.value !== "recent" || this.peek()?.image !== "(")
            this.fail("Use recent(occurred|observed|valid|formed|recorded)");
          this.take("(");
          const axis = this.name();
          if (
            !["occurred", "observed", "valid", "formed", "recorded"].includes(
              axis,
            )
          )
            this.fail("Unknown recent time axis");
          this.take(")");
          operand.value = `recent:${axis}`;
        }
        if (operand.kind === "universe")
          this.fail("Universe is not a preference");
        if (parenthesized) this.take(")");
        left.preferences.push({ negative: symbol === "-", operand });
        continue;
      }
      const precedence = symbol === "&&" ? 2 : symbol === "||" ? 1 : 0;
      if (precedence === 0 || precedence <= minimum) break;
      this.take();
      const right = this.query(precedence);
      left = {
        operation: symbol === "&&" ? "all" : "any",
        children: [left, right],
        directives: [],
        preferences: [],
      };
    }
    this.depth--;
    return left;
  }
  private atom(): Atom {
    const token = this.peek();
    if (token?.tokenType === Quoted)
      return { kind: "text", text: this.string() };
    if (token?.image === "*") {
      this.take();
      return { kind: "universe" };
    }
    if (token?.image === "#") {
      this.take();
      let text = this.name();
      while (
        this.peek()?.image === "-" &&
        this.tokens[this.index + 1]?.tokenType === Name &&
        this.peek()!.startOffset === this.tokens[this.index - 1]!.endOffset! + 1
      ) {
        this.take("-");
        text += `-${this.name()}`;
      }
      return { kind: "concept", text };
    }
    if (token?.image !== "@") this.fail("Expected query atom");
    this.take("@");
    const selector = this.name();
    if (!["e", "tag", "schema", "r", "object", "ref"].includes(selector))
      this.fail("Unknown selector");
    this.take("(");
    const locators: Locator[] = [];
    do {
      if (locators.length) this.take(",");
      if (this.peek()?.tokenType === Quoted)
        locators.push({ kind: "name", value: this.string() });
      else if (this.peek()?.tokenType === Lexical)
        locators.push({ kind: "lexical", value: this.take().image });
      else this.fail("Expected name or typed lexical reference");
    } while (this.peek()?.image === ",");
    this.take(")");
    if (locators.length > 32 || (selector !== "e" && locators.length !== 1))
      this.fail("Invalid selector arity");
    if (selector === "ref" && locators[0]?.kind !== "lexical")
      this.fail("@ref requires a LexicalRef");
    if (selector === "object" && locators[0]?.kind !== "name")
      this.fail("@object requires an opaque quoted Host ref");
    if (!isSelector(selector)) this.fail("Unknown selector");
    return {
      kind: "selector",
      selector,
      locators,
    };
  }
  private directive(): Directive {
    this.take("$");
    const name = this.name();
    const output: Directive = { name, positional: [], named: {} };
    if (this.peek()?.image !== "(") return output;
    this.take("(");
    if (this.peek()?.image === ")") {
      this.take();
      return output;
    }
    while (true) {
      if (
        this.peek()?.tokenType === Name &&
        this.tokens[this.index + 1]?.image === "="
      ) {
        const key = this.name();
        this.take("=");
        if (Object.hasOwn(output.named, key))
          this.fail("Duplicate named argument");
        output.named[key] = this.argument();
      } else {
        if (Object.keys(output.named).length)
          this.fail("Positional argument after named argument");
        output.positional.push(this.argument());
      }
      if (this.peek()?.image !== ",") break;
      this.take(",");
    }
    this.take(")");
    return output;
  }
  private argument(): Argument {
    if (this.peek()?.tokenType === Quoted) return this.string();
    if (this.peek()?.tokenType === NumberToken)
      return Number(this.take().image);
    if (
      this.peek()?.tokenType === Duration ||
      this.peek()?.tokenType === Name ||
      this.peek()?.tokenType === Lexical
    )
      return this.take().image;
    return this.fail("Invalid directive argument");
  }
  private name(): string {
    if (this.peek()?.tokenType !== Name) this.fail("Expected name");
    return this.take().image;
  }
  private string(): string {
    try {
      const value: unknown = JSON.parse(this.take().image);
      if (typeof value !== "string") this.fail("Invalid quoted string");
      return value;
    } catch {
      return this.fail("Invalid quoted string escape");
    }
  }
}

export function canonical(e: Expression): string {
  const base =
    e.operation === "atom"
      ? atomText(e.atom!)
      : `(${e.children.map(canonical).join(e.operation === "all" ? " && " : " || ")})`;
  const directives = [...e.directives]
    .sort(
      (a, b) =>
        a.name.localeCompare(b.name) ||
        (a.name === "time"
          ? ["occurred", "observed", "valid", "formed", "recorded"].indexOf(
              String(a.positional[0]),
            ) -
            ["occurred", "observed", "valid", "formed", "recorded"].indexOf(
              String(b.positional[0]),
            )
          : 0),
    )
    .map(
      (d) =>
        `$${d.name}${
          d.positional.length || Object.keys(d.named).length
            ? `(${[
                ...(d.name === "return"
                  ? [...d.positional].sort(
                      (a, b) =>
                        resultDomainOrder.indexOf(String(a)) -
                        resultDomainOrder.indexOf(String(b)),
                    )
                  : d.positional
                ).map(argumentText),
                ...Object.entries(d.named)
                  .sort(([a], [b]) => a.localeCompare(b))
                  .map(([k, v]) => `${k}=${argumentText(v)}`),
              ].join(",")})`
            : ""
        }`,
    );
  const preferences = e.preferences.map(
    (p) =>
      `${p.negative ? "-" : "+"}${p.operand.kind === "key" ? p.operand.value.replace(/^recent:(.+)$/, "recent($1)") : atomText(p.operand)}`,
  );
  return [base, ...directives, ...preferences].join(" ");
}
function argumentText(v: Argument) {
  return typeof v === "number"
    ? String(v)
    : /^[a-z_][a-z0-9_]*$/.test(v) || /^\d+(ms|s|m|h|d|w)$/.test(v)
      ? v
      : JSON.stringify(v);
}
function atomText(a: Atom): string {
  if (a.kind === "universe") return "*";
  if (a.kind === "text") return JSON.stringify(a.text);
  if (a.kind === "concept") return `#${a.text}`;
  return `@${a.selector}(${a.locators.map((l) => (l.kind === "name" ? JSON.stringify(l.value) : l.value)).join(",")})`;
}
