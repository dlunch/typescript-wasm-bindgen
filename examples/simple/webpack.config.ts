import * as webpack from 'webpack';
import * as path from 'path';
import * as WasmPackPlugin from '@wasm-tool/wasm-pack-plugin';
import TsconfigPathsPlugin from 'tsconfig-paths-webpack-plugin';
import * as HtmlEntryLoader from 'html-entry-loader';

const dist = path.resolve('dist');

const configuration: webpack.Configuration = {
  mode: 'development',
  entry: {
    index: 'src/index.html',
  },
  output: {
    path: dist,
    filename: '[name].js',
  },
  experiments: {
    asyncWebAssembly: true
  },
  module: {
    rules: [
      {
        test: /\.(html)$/,
        use: [
          {
            loader: 'html-entry-loader',
            options: {
              minimize: true,
            },
          },
        ],
      },
      {
        test: /\.ts$/,
        use: [
          {
            loader: 'ts-loader',
            options: {
              onlyCompileBundledFiles: true,
              compilerOptions: {
                module: 'esnext',
              },
            },
          },
        ],
      },
    ],
  },
  resolve: {
    extensions: ['.ts', '.js'],
    plugins: [new TsconfigPathsPlugin() as any],
    modules: ['src']
  },
  plugins: [
    new HtmlEntryLoader.EntryExtractPlugin(),

    new (WasmPackPlugin as any)({
      crateDirectory: path.resolve('wasm'),
      outDir: path.resolve('wasm/pkg'),
      outName: 'index',
    })
  ],
};

export default configuration;
