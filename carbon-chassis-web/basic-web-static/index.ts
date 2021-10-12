
// const rust = import('./dist/carbon_chassis_web');
import {run} from './dist/carbon_chassis_web';

//
// async function getWasm() {
//
// }
// console.log("JS loaded");
// rust
//   .then(m => m.run())
//   .catch(console.error);


function start(mymod: typeof import('./dist/carbon_chassis_web')) {
    console.log("All modules loaded");
    mymod.run();
}

async function load() {
    start(await import('./dist/carbon_chassis_web'));
}

load();