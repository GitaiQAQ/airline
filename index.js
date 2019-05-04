// For more comments about what's going on here, check out the `hello_world`
// example.
const rust = import('./pkg/wasm');

// rust
//     .then(m => {
//         window.ctx = document.getElementsByTagName("canvas")[0].getContext("2d");
//         window.from = m.Position.new(50.0, 50.0);
//         window.to = m.Position.new(350.0, 100.0);
//         window.color = m.HSL.new(255,1.0,0.0);
//         Object.assign(window, m);
//         // for (let i = 1; i <= 10; i++) {
//         //     draw_air_line(ctx, from, to, color, 0.3, i / 10)
//         // }
//     })
//     .catch(console.error);