export type Email = string;

export function parseEmail(text: string): Email {
  if (!text.includes('@')) throw new Error(`${text} is not an email address`);
  return text;
}
