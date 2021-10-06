const rust = import('./dist/carbon_chassis_web');
console.log("JS loaded")
rust
  .then(m => m.run())
  .catch(console.error);
