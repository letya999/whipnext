"use strict";

module.exports = [
  {
    files: ["assets/ui/**/*.js"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "script",
      globals: {
        window: "readonly",
        document: "readonly",
        console: "readonly",
        fetch: "readonly",
        Audio: "readonly",
        FileReader: "readonly",
        Promise: "readonly",
        Uint8Array: "readonly",
        Uint16Array: "readonly",
        Uint32Array: "readonly",
        Float32Array: "readonly",
        DataView: "readonly",
        TextDecoder: "readonly",
        performance: "readonly",
        requestAnimationFrame: "readonly",
        cancelAnimationFrame: "readonly",
        setTimeout: "readonly",
        clearInterval: "readonly",
        setInterval: "readonly",
      },
    },
    rules: {
      "no-undef": "error",
      "no-unused-vars": ["error", { args: "none", caughtErrors: "none" }],
      "no-var": "off",
    },
  },
];
