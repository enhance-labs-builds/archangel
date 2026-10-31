// The types of an archangel plugin: the view its conditions receive, and the helpers that declare a
// plugin and its conditions. The view's fields match what archangel sends; a test in the engine
// compares the two. A plugin reads only the fields it knows, so a field added to the view never
// breaks one, and `schemaVersion` changes only when a field's meaning does.

/** What a condition's `check` receives: the files in its rule's `files`. */
export interface View {
  /** The version of the view, which a plugin declares it was written for. */
  readonly schemaVersion: 1;
  /** The files, sorted by path, each with the facts its condition reads. */
  readonly files: readonly FileRecord[];
}

/** One file of the view. */
export interface FileRecord {
  /** The path, relative to the config folder. */
  readonly path: string;
  /** The file's element, or `null` when it is in none. */
  readonly element: string | null;
  /** The values of the element's captures, by name. */
  readonly captures: Readonly<Record<string, string>>;
  /** The declared selectors the file is in. */
  readonly selectors: readonly string[];
  /** The file's imports, for a condition that reads `imports`. */
  readonly imports?: readonly ImportRecord[];
  /** The paths of the files that import it, for a condition that reads `importers`. */
  readonly importers?: readonly string[];
  /** What the file exports, for a condition that reads `exports`. */
  readonly exports?: ExportsRecord;
}

/** One import of a file. */
export interface ImportRecord {
  /** The specifier, as written. */
  readonly specifier: string;
  /** Which import of that specifier in the file, from 0. */
  readonly occurrence: number;
  /** The line, from 1. */
  readonly line: number;
  /** The column, from 1, in characters. */
  readonly column: number;
  /** `runtime`, or `type` for an import the compiler removes. */
  readonly kind: 'runtime' | 'type';
  /** The names it brings in: `default`, `*` or the name. */
  readonly names: readonly string[];
  /** The repository file it resolves to, or `null`. */
  readonly file: string | null;
  /** The package it resolves to, or `null`. */
  readonly package: string | null;
}

/** What a file exports. */
export interface ExportsRecord {
  /** The names it exports, `default` included. */
  readonly names: readonly string[];
  /** Whether it has a default export. */
  readonly default: boolean;
  /** Whether it has an `export *`. */
  readonly star: boolean;
}

/** A fact of a file that a condition may read. */
export type Fact = 'imports' | 'importers' | 'exports';

/** A file of the view of a condition that reads the facts `R`, which are then always there. */
export type FileWith<R extends Fact> = Omit<FileRecord, Fact> & {
  readonly [K in R]-?: NonNullable<FileRecord[K]>;
};

/** The view of a condition that reads the facts `R`. */
export type ViewWith<R extends Fact> = Omit<View, 'files'> & { readonly files: readonly FileWith<R>[] };

/** One violation a condition reports: a file of its view, the import if it is about one, and why. */
export type Violation = {
  /** A file of the condition's view; any other file is a defect of the plugin. */
  readonly file: string;
  /** The import the violation is about, which an exception above it can permit. */
  readonly import?: { readonly specifier: string; readonly occurrence?: number };
  /** What is wrong, for the developer. */
  readonly message: string;
};

/** A condition: what a rule's files must do, which archangel cannot say with its own conditions. */
export type Condition<N extends string = string, O = unknown, R extends Fact = Fact> = {
  /** The name a rule uses after the plugin's name, as `team/<name>`. */
  readonly name: N;
  /** What the condition requires, in one sentence. */
  readonly description: string;
  /** Where to read more. */
  readonly docs?: string;
  /** The JSON Schema of the options, which archangel checks each rule's options against. */
  readonly optionsSchema?: object | null;
  /** The facts of each file the condition reads; the view holds only these. */
  readonly reads: readonly R[];
  /** The violations in the view, given the rule's options. */
  check(view: ViewWith<R>, options: O): readonly Violation[] | Promise<readonly Violation[]>;
};

/** Any condition, whatever its options and facts. */
export type AnyCondition = Condition<string, never, Fact>;

/** A plugin: a name, the view version it was written for, and its conditions. */
export type Plugin<N extends string = string, C extends readonly AnyCondition[] = readonly AnyCondition[]> = {
  /** The plugin's name, by convention its package's without `archangel-plugin-`. */
  readonly name: N;
  /** The version of the view the plugin was written for. */
  readonly schemaVersion: 1;
  /** The conditions. */
  readonly conditions: C;
};

/** Declares a condition, with types for its options and for the facts it reads. */
export declare function defineCondition<const N extends string, O = unknown, const R extends Fact = never>(
  condition: Condition<N, O, R>,
): Condition<N, O, R>;

/** Declares a plugin, which a config loads under a name in `plugins`. */
export declare function definePlugin<
  const N extends string,
  const C extends readonly AnyCondition[],
>(plugin: Plugin<N, C>): Plugin<N, C>;
