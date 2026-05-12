/* eslint-env node */
module.exports = {
  root: true,
  parser: "@typescript-eslint/parser",
  parserOptions: {
    ecmaVersion: 2022,
    sourceType: "module",
    ecmaFeatures: { jsx: true },
  },
  env: {
    browser: true,
    es2022: true,
  },
  plugins: ["@typescript-eslint", "solid"],
  extends: [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
    "plugin:solid/typescript",
  ],
  rules: {
    "@typescript-eslint/no-unused-vars": [
      "error",
      { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
    ],
    // Solid components are functions, but using non-PascalCase identifiers as
    // JSX tags is a footgun.
    "no-restricted-syntax": "off",
  },
  ignorePatterns: ["dist/", "node_modules/", "*.cjs", "vite.config.ts"],
};
