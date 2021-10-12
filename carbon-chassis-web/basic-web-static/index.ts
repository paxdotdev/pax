
// const rust = import('./dist/carbon_chassis_web');
import {CarbonChassisWeb} from './dist/carbon_chassis_web';

//
// async function getWasm() {
//
// }
// console.log("JS loaded");
// rust
//   .then(m => m.run())
//   .catch(console.error);


function main(wasmMod: typeof import('./dist/carbon_chassis_web')) {
    console.log("All modules loaded");
    let ccw = wasmMod.CarbonChassisWeb.new();

    // ccw.init()
    // let ccw = wasmMod.CarbonChassisWeb.;
    // wasmMod.init();

    requestAnimationFrame(renderLoop.bind(renderLoop, ccw))

    // console.log("Retrieving test data", wasmMod.retrieve_test_data());
}

async function load() {
    main(await import('./dist/carbon_chassis_web'));
}

load().then();


function renderLoop (ccw: CarbonChassisWeb) {
     let messages = ccw.tick();
     console.log("Got messages", messages);
     requestAnimationFrame(renderLoop.bind(renderLoop, ccw))
}


//TODO:  traverse through render_message_queue after each engine tick
//       render those messages as appropriate


//TODO:  should we port the request_animation_frame => tick logic
//       to live in ts instead of rust?
//       1. it's far cleaner to invoke rAF from TS
//       2. it should make it clean/clear how to pass data (tick() returns the MQ,
//          ... Can even receive an MQ for inbounds/input `tick(inbound_mq)`)