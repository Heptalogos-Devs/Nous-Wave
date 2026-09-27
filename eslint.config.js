import oxlint from "eslint-plugin-oxlint";

export default [
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/build/**",
      "**/coverage/**",
      "**/generated/**",
      "**/*.ts",
      "**/*.tsx",
      "**/*.proto",
    ],
  },
  ...oxlint.configs["flat/recommended"],
];
