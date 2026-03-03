import xmlFormatter from "xml-formatter";
import { parse, print } from "graphql";

export function formatJson(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

export function formatXml(text: string): string {
  try {
    return xmlFormatter(text, { indentation: "  ", collapseContent: true, lineSeparator: "\n" });
  } catch {
    return text;
  }
}

export function formatGraphql(text: string): string {
  try {
    return print(parse(text));
  } catch {
    return text;
  }
}
