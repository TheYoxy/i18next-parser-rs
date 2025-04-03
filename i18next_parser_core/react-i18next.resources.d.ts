
/// This file is generated automatically
/// All changes will be lost
/* eslint-disable */

import 'i18next';

import type namespace_en from 'en/namespace.json';
import type anotherNamespace_en from 'en/another_namespace.json';

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'default';
    returnNull: false;
    returnObjects: false;
    nsSeparator: ':';
    keySeparator: '.';
    contextSeparator: '_';
    jsonFormat: 'v4';
    allowObjectInHTMLChildren: false;
    resources: {
      namespace: typeof namespace_en;
      'another_namespace': typeof anotherNamespace_en;
      default: {}
    };
  }

  interface Resource {
    en: {
namespace: typeof namespace_en;
        'another_namespace': typeof anotherNamespace_en;
},

  }
}

declare global {
  type Ns = 'another_namespace' | 'namespace';
}

export default {};
