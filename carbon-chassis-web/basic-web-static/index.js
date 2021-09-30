const rust = import('./dist/carbon_example');
console.log("JS loaded")
rust
  .then(m => m.run())
  .catch(console.error);
