const path = require('path');

module.exports = {
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
      {
        test: /\.wasm$/,
        type: 'webassembly/async',
      },
      {
        test: /\.(jpe?g|svg|png|gif|ico|eot|ttf|otf|woff2?)(\?v=\d+\.\d+\.\d+)?$/i,
        type: 'asset/resource',
      },
    ],
  },

  resolve: {
    extensions: ['.tsx', '.html', '.ts', '.js', '.wasm', '.css'],
  },

  devServer: {
    host: '0.0.0.0',
    static: {
      directory: path.resolve(__dirname, 'public'),
    },
  },

  entry: './src/index.ts',

  output: {
    path: path.join(path.resolve(__dirname), 'dist'),
    filename: 'index.js',
    publicPath: '/',
  },

  plugins: [],

  experiments: {
    asyncWebAssembly: true,
  },

  mode: 'production',
};