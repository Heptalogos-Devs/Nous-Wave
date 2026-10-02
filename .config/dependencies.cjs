module.exports = {
  forbidden: [
    {
      name: "no-circular",
      severity: "error",
      from: {},
      to: { circular: true },
    },
    {
      name: "core-does-not-import-kernel",
      severity: "error",
      from: { path: "^apps/nous-core/src" },
      to: { path: "^apps/nous-kernel|^crates" },
    },
  ],
  options: {
    parser: "swc",
    doNotFollow: ["generated"],
    exclude: "(^|/)(node_modules|dist|build|coverage|generated)(/|$)",
    moduleSystems: ["cjs", "es6"],
  },
};
