# i18next-parser-rs

This project aims to be a fully compatible rewrite of [i18next-parser](https://github.com/i18next/i18next-parser)

[Changelog](./CHANGELOG.md)
## Cli
Here is the cli definition:
```bash
$ i18next-parser --help

A simple i18next parser

Usage: i18next-parser [OPTIONS] [PATH]

Arguments:
  [PATH]  The path to extract the translations from [default: `pwd`/i18next-parser-rs]

Options:
  -v, --verbose
          Should the output to be verbose
  -g, --generate-types
          Should generate types
      --generate-shell <GENERATE_SHELL>
          Should generate shell completions [possible values: bash, elvish, fish, powershell, zsh]
  -h, --help
          Print help
  -V, --version
          Print version
```

### Arguments
#### --generate-types | -g
`-g` option provides a way to generate a `.d.ts` file used to get automatic auto completion from typescript types.

This file will be named `react-i18next.resources.d.ts` and will be located at the `PATH` provided location.

example:
```typescript
// This file is generated automatically
// All changes will be lost
import 'i18next';

import type translation from 'public/locales/en/translation.json';

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'translation';
    returnNull: false;
    returnObjects: false;
    nsSeparator: ':'; // provided from options
    keySeparator: '.'; // provided from options
    contextSeparator: '_'; // provided from options
    jsonFormat: 'v4';
    allowObjectInHTMLChildren: false;
    resources: {
      translation: typeof translation;
    };
  }
}

declare global {
  type Ns =
    | 'translation';
}
```

### Options

```json
{
  "contextSeparator": "_",
  "createOldCatalogs": true,
  "defaultNamespace": "default",
  "indentation": 2,
  "keepRemoved": false,
  "keySeparator": ".",
  "lineEnding": "lf",
  "locales": ["en", "fr"],
  "namespaceSeparator": ":",
  "output": "public/locales/$LOCALE/$NAMESPACE.json",
  "pluralSeparator": "_",
  "input": ["app/**/*.{js,jsx,ts,tsx}"],
  "sort": true,
  "verbose": false,
  "failOnWarnings": true,
  "failOnUpdate": false,
  "customValueTemplate": null,
  "resetDefaultValueLocale": null
}
```

### Format
as is, only the i18next v4 format is supported.
