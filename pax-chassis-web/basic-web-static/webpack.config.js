const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');

module.exports = {
  module: {
      rules: [
        {
          test: /\.tsx?$/,
          use: 'ts-loader',
          exclude: /node_modules/
        },
        {
          test: /\.wasm$/,
          type: "webassembly/experimental"
        }
      ]
    },
  resolve: {
    extensions: [ '.tsx', '.ts', '.js', '.wasm' ]
  },
  entry: './index.ts',
  output: {
    path: path.resolve(__dirname),
    filename: 'index.js',
  },
  plugins: [
    // Have this example work in Edge which doesn't ship `TextEncoder` or
    // `TextDecoder` at this time.
    new webpack.ProvidePlugin({
                              TextDecoder: ['text-encoding', 'TextDecoder'],
                              TextEncoder: ['text-encoding', 'TextEncoder']
    })
  ],
  mode: 'development'
};
