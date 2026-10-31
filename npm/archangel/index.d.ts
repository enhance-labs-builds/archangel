// The types of an archangel config. They check, as the config is written, that every rule names a
// declared element or selector, that a sentence has exactly one condition, and that an isolate
// rule names an element with a capture such as `{feature}`. The names inside `selectors` are
// left to archangel, because checking them here makes TypeScript infer mutable lists for them.

import type { AnyCondition, Plugin } from './plugin.js';

export * from './plugin.js';

/** A pattern, a list of patterns, or the packages of a package element. */
export type ElementSpec = string | readonly string[] | { readonly packages: readonly string[] };

/** The elements, by name. */
export type Elements = Record<string, ElementSpec>;

/** Whether a pattern has a capture: braces around a name, not a comma-separated alternative. */
type PatternCaptures<P extends string> = P extends `${string}{${infer Inside}}${infer Rest}`
  ? Inside extends `${string},${string}`
    ? PatternCaptures<Rest>
    : true
  : false;

/** Whether an element's pattern, or any of its patterns, has a capture. */
type Captures<S> = S extends string
  ? PatternCaptures<S>
  : S extends readonly (infer P extends string)[]
    ? true extends PatternCaptures<P>
      ? true
      : false
    : false;

/** The name of a declared element. */
export type ElementName<E extends Elements> = Extract<keyof E, string>;

/** The name of a declared element of files, not of packages. */
export type FileElementName<E extends Elements> = {
  [K in ElementName<E>]: E[K] extends { readonly packages: readonly string[] } ? never : K;
}[ElementName<E>];

/** The name of a declared element whose pattern has a capture, as isolate needs. */
export type CapturedElement<E extends Elements> = {
  [K in ElementName<E>]: Captures<E[K]> extends true ? K : never;
}[ElementName<E>];

/** A path pattern: a string with a `/` or a `*`, which no declared name needs. */
export type PathPattern = `${string}/${string}` | `${string}*${string}`;

/**
 * Which files a selector or a rule is about: a declared name `N`, a path pattern, a list of these
 * (any of them), or one combination.
 */
export type SelectorDefinition<N extends string, F extends string = N> =
  | N
  | PathPattern
  | readonly SelectorDefinition<N, F>[]
  | SelectorObject<N, F>;

/** The combinations a selector may use, each an object of one shape. */
export type SelectorObject<N extends string, F extends string = N> =
  | { readonly all: readonly SelectorDefinition<N, F>[]; readonly any?: never; readonly not?: never }
  | { readonly any: readonly SelectorDefinition<N, F>[]; readonly all?: never; readonly not?: never }
  | { readonly not: SelectorDefinition<N, F>; readonly all?: never; readonly any?: never }
  /** The files of one element, `'*'` for the files of any element. */
  | { readonly element: F | '*'; readonly capture?: never; readonly directive?: never }
  /** The files of one instance of an element, such as `{ element: 'feature', capture: { feature: 'login' } }`. */
  | {
      readonly element: F;
      readonly capture: Readonly<Record<string, string>>;
      readonly directive?: never;
    }
  /** The files that start with a directive, such as `'use client'`. */
  | { readonly directive: string; readonly element?: never };

/** The selectors, by name. */
export type Selectors = Record<string, SelectorDefinition<string>>;

/** A name a selector or a sentence's `files` may use: a file element or a selector. */
type FilesName<E extends Elements, S extends Selectors> = FileElementName<E> | Extract<keyof S, string>;

/** Which files a sentence is about. */
export type Files<E extends Elements, S extends Selectors> = SelectorDefinition<
  NoInfer<FilesName<E, S>>,
  NoInfer<FileElementName<E>>
>;

/** One thing on the other side of an import condition. */
export type SideItem<E extends Elements, S extends Selectors> =
  | NoInfer<ElementName<E> | Extract<keyof S, string>>
  | PathPattern
  /** Packages by name; a name may end in `*`, and `'*'` is every package. */
  | { readonly packages: readonly string[] }
  | SelectorObject<NoInfer<FilesName<E, S>>, NoInfer<FileElementName<E>>>;

/** The other side of an import condition: one thing, or a list of them. */
export type Side<E extends Elements, S extends Selectors> =
  | SideItem<E, S>
  | readonly SideItem<E, S>[];

/** Files by path, relative to the config folder, with their text. */
export type ExampleFiles = Readonly<Record<string, string>>;

/** What every rule may have. */
export interface RuleOptions {
  /** The rule's name, for exceptions and the baseline; each rule gives a default. */
  readonly name?: string;
  /** Why the rule exists, shown with each violation. */
  readonly reason?: string;
  /** Where to read more, shown with each violation. */
  readonly link?: string;
  /** Which imports the rule checks: all of them, those that stay at runtime, or type imports. */
  readonly imports?: 'all' | 'runtime' | 'types';
  /** File sets the rule must accept and must reject, which `archangel test` checks. */
  readonly examples?: {
    readonly accepted?: readonly ExampleFiles[];
    readonly rejected?: readonly ExampleFiles[];
  };
}

/** What limits an import condition to some imports. */
export interface ImportLimits {
  /** Only imports of these names. */
  readonly names?: readonly string[];
  /**
   * Only imports written this way: starting with `./` or `../`, starting with `../`, through a
   * tsconfig path or a package `imports` field, or as a package name.
   */
  readonly specifier?: 'relative' | 'parent' | 'alias' | 'package';
  /** Only imports between the same instance, or between different instances, of `files`. */
  readonly instance?: 'same' | 'different';
  /**
   * Only imports of packages that the importing file's nearest package.json lists only as
   * development dependencies, or does not list at all.
   */
  readonly dependency?: 'dev' | 'undeclared';
}

/** A way of writing the words of a name. */
export type NameStyle = 'kebab-case' | 'camelCase' | 'PascalCase' | 'snake_case' | 'SCREAMING_SNAKE_CASE';

/**
 * A name: text, `*` for any text, and `{FileName}`, `{fileName}`, `{file-name}`, `{file_name}` or
 * `{FILE_NAME}` for the file's name in each style, or the same spellings of `{FolderName}` for the
 * name of the folder that holds it.
 */
export type NamePattern = string;

type Conditions<Other> = {
  /** The files may not import the other side. */
  readonly mayNotImport: Other;
  /** The files may import only the other side. */
  readonly mayOnlyImport: Other;
  /** Only the other side may import the files. */
  readonly mayOnlyBeImportedBy: Other;
  /**
   * The name, up to the first dot, is in a style, or the full name, without the last extension,
   * matches a name pattern.
   */
  readonly mustBeNamed: NameStyle | (NamePattern & {});
  /**
   * Each instance of `files`, an element with a capture, has a file that matches the pattern, or
   * one of the patterns.
   */
  readonly mustContain: string | readonly string[];
  /** No file may match `files`. */
  readonly mustNotExist: true;
  /** Each file exports a name that matches the pattern; re-exports and `export *` count. */
  readonly mustExport: NamePattern;
  /** No file has a default export, an `export *`, or a named export that matches the pattern. */
  readonly mayNotExport: 'default' | 'star' | (NamePattern & {});
  /** Each file holds only imports, exports from other files and `export { … }` of imports. */
  readonly mustOnlyReExport: true;
  /** The files may not import each other in a cycle. */
  readonly noCycles: true;
};

/** Condition `C`, with every other condition ruled out. */
type Only<Other, C extends keyof Conditions<Other>> = Pick<Conditions<Other>, C> & {
  readonly [K in Exclude<keyof Conditions<Other>, C>]?: never;
};

type NoLimits = { readonly [K in keyof ImportLimits]?: never };

type NoChains = { readonly transitively?: false; readonly stopAt?: never };

/** A rule written as a sentence: `files`, and exactly one condition. */
export type Sentence<E extends Elements, S extends Selectors> = Omit<RuleOptions, 'imports'> & {
  readonly kind?: never;
  /** The files the rule is about. */
  readonly files: Files<E, S>;
} & (
    | (Only<Side<E, S>, 'mayNotImport'> & ImportLimits & NoChains & Pick<RuleOptions, 'imports'>)
    | (Only<Side<E, S>, 'mayNotImport'> &
        NoLimits & {
          /** Follow chains of imports, not only direct ones. */
          readonly transitively: true;
          /** Files where a chain ends, such as `{ directive: 'use server' }`. */
          readonly stopAt?: Files<E, S>;
          /** A chain of type imports never runs, so a transitive rule checks no type imports. */
          readonly imports?: 'all' | 'runtime';
        })
    | (Only<Side<E, S>, 'mayOnlyImport'> & ImportLimits & NoChains & Pick<RuleOptions, 'imports'>)
    | (Only<Side<E, S>, 'mayOnlyBeImportedBy'> &
        ImportLimits &
        NoChains &
        Pick<RuleOptions, 'imports'>)
    | (Only<Side<E, S>, 'mustBeNamed'> & NoLimits & NoChains & NoImports)
    | (Only<Side<E, S>, 'mustContain'> & NoLimits & NoChains & NoImports)
    | (Only<Side<E, S>, 'mustNotExist'> & NoLimits & NoChains & NoImports)
    | (Only<Side<E, S>, 'mustExport'> & NoLimits & NoChains & NoImports)
    | (Only<Side<E, S>, 'mayNotExport'> & NoLimits & NoChains & NoImports)
    | (Only<Side<E, S>, 'mustOnlyReExport'> & NoLimits & NoChains & NoImports)
    | (Only<Side<E, S>, 'noCycles'> & NoLimits & NoChains & Pick<RuleOptions, 'imports'>)
  );

type NoImports = { readonly imports?: never };

/** A rule about folders: `folders`, and `mustBeNamed` or `mustContain`. */
export type FolderSentence = Omit<RuleOptions, 'imports'> & {
  readonly kind?: never;
  readonly files?: never;
  /**
   * The folders the rule is about: those that one of these patterns matches, below the
   * pattern's fixed part, so `src/**` is every folder inside `src`. A pattern that starts with
   * `!` leaves the folders it matches out.
   */
  readonly folders: string | readonly string[];
} & NoLimits &
  NoChains &
  NoImports &
  (
    | (Only<never, 'mustBeNamed'>)
    | (Only<never, 'mustContain'>)
    | (Only<never, 'mustNotExist'>)
  );

/** A sentence of a rule of several sentences: the rule holds its name, reason and examples. */
type InnerSentence<T> = T extends unknown ? Omit<T, 'name' | 'reason' | 'link' | 'examples' | 'imports'> : never;

/** A shorthand as one sentence of a rule of several sentences. */
type InnerShorthand<E extends Elements> =
  | { readonly kind: 'layers'; readonly order: readonly NoInfer<ElementName<E>>[] }
  | { readonly kind: 'isolate'; readonly element: NoInfer<CapturedElement<E>> }
  | { readonly kind: 'entry'; readonly element: NoInfer<ElementName<E>>; readonly files: readonly string[] };

/** A rule of several sentences, broken when any of them is, which needs a name. */
export type SeveralSentences<E extends Elements, S extends Selectors> = Omit<RuleOptions, 'name'> & {
  readonly kind?: never;
  readonly files?: never;
  /** The rule's name, for exceptions and the baseline. */
  readonly name: string;
  /** The sentences, each about `files` or `folders` with one condition, or a shorthand. */
  readonly sentences: readonly (InnerSentence<Sentence<E, S> | FolderSentence> | InnerShorthand<E>)[];
};

/** The plugins a config loads, by the name its rules use them by. */
export type Plugins = Readonly<Record<string, Plugin>>;

/** The name of a condition of a loaded plugin, as a rule writes it: `<plugin>/<condition>`. */
export type ConditionName<P extends Plugins> = {
  [K in Extract<keyof P, string>]: P[K]['conditions'][number] extends infer C
    ? C extends AnyCondition
      ? `${K}/${C['name']}`
      : never
    : never;
}[Extract<keyof P, string>];

/** A rule that uses a plugin's condition, whose options the condition's schema checks. */
export type PluginRule<E extends Elements, S extends Selectors, P extends Plugins> = Omit<
  RuleOptions,
  'imports'
> & {
  readonly kind?: never;
  /** The files whose records the condition sees. */
  readonly files: Files<E, S>;
  /** The condition, as `<plugin>/<condition>`. */
  readonly plugin: NoInfer<ConditionName<P>>;
  /** The options, which the condition's JSON Schema checks when the config loads. */
  readonly options?: unknown;
};

/** A rule, as an object: one of the three shorthands, a sentence, or a plugin rule. */
export type Rule<E extends Elements, S extends Selectors = {}, P extends Plugins = {}> =
  | (RuleOptions & { readonly kind: 'layers'; readonly order: readonly NoInfer<ElementName<E>>[] })
  | (RuleOptions & { readonly kind: 'isolate'; readonly element: NoInfer<CapturedElement<E>> })
  | (RuleOptions & {
      readonly kind: 'entry';
      readonly element: NoInfer<ElementName<E>>;
      readonly files: readonly string[];
    })
  | Sentence<E, S>
  | FolderSentence
  | SeveralSentences<E, S>
  | PluginRule<E, S, P>;

/** The rule builders that a `rules` function receives. */
export interface RuleBuilders<E extends Elements, S extends Selectors = {}, P extends Plugins = {}> {
  /** Each listed element may import only itself and later listed elements. */
  layers(order: readonly NoInfer<ElementName<E>>[], options?: RuleOptions): Rule<E, S, P>;
  /** The instances of the element may not import each other. */
  isolate(element: NoInfer<CapturedElement<E>>, options?: RuleOptions): Rule<E, S, P>;
  /** Code outside an instance of the element may import it only through these files. */
  entry(
    element: NoInfer<ElementName<E>>,
    files: string | readonly string[],
    options?: RuleOptions,
  ): Rule<E, S, P>;
  /** A rule as a sentence: `files`, or `folders`, and exactly one condition or plugin condition. */
  rule(
    sentence: Sentence<E, S> | FolderSentence | SeveralSentences<E, S> | PluginRule<E, S, P>,
  ): Rule<E, S, P>;
}

/**
 * What a preset returns: part of a config, which `extends` puts under the config. Archangel names
 * each element, selector and rule `<name>/<its name>`, and reads paths relative to `root`.
 */
export interface Preset {
  /** The prefix of everything the preset declares, such as `fsd`. */
  readonly name: string;
  /** The package the preset comes from, which a problem its user cannot fix names. */
  readonly package?: string;
  /** The folder the preset applies to, relative to the config folder. */
  readonly root?: string;
  readonly elements?: Elements;
  readonly selectors?: Selectors;
  readonly plugins?: Plugins;
  /** The rules, each with a name. */
  readonly rules?: readonly unknown[] | ((builders: RuleBuilders<Elements, Selectors, Plugins>) => readonly unknown[]);
}

/** The options of the Feature-Sliced Design preset. */
export interface FsdOptions {
  /** The folder that holds the layers, such as `src`. */
  readonly root: string;
  /** Turn on Steiger's four opt-in rules too. */
  readonly strict?: boolean;
  /** The prefix, for a second use in one repository; `fsd` by default. */
  readonly name?: string;
}

/** The Feature-Sliced Design preset: Steiger's rules for the layers under `root`. */
export declare function fsd(options: FsdOptions): Preset;

/** The Next.js preset: code a client component reaches stays out of server-only code. */
export declare function nextjs(options?: { readonly name?: string; readonly root?: string }): Preset;

/** An archangel config. */
export interface Config<E extends Elements, S extends Selectors = {}, P extends Plugins = {}> {
  /** The elements: parts of the code by file pattern, or packages; a config that extends a preset may have none. */
  readonly elements?: E;
  /** Named sets of files, which rules and other selectors use by name. */
  readonly selectors?: S;
  /** Lines in `.gitignore` syntax that remove files from the check. */
  readonly ignore?: readonly string[];
  /** Specifiers that may stay unresolved, each with the reason; a specifier may end in `*`. */
  readonly allowUnresolved?: Readonly<Record<string, string>>;
  /** The plugins, by the name the rules use them by, as `plugin: '<name>/<condition>'`. */
  readonly plugins?: P;
  /** The presets the config extends; the config's own rules replace preset rules of one name. */
  readonly extends?: readonly Preset[];
  /** Rule names to turn off after the presets are merged, such as `fsd/insignificant-slice`. */
  readonly off?: readonly string[];
  /** The rules, or a function that builds them. */
  readonly rules:
    | readonly Rule<E, S, P>[]
    | ((builders: RuleBuilders<E, S, P>) => readonly Rule<E, S, P>[]);
}

/** Declares an archangel config, with types that check its names as it is written. */
export declare function defineConfig<
  const E extends Elements,
  const S extends Selectors = {},
  const P extends Plugins = {},
>(config: Config<E, S, P>): Config<E, S, P>;
