const esbuild = require('esbuild');
const fs = require('fs');
const path = require('path');

const production = process.argv.includes('--production');
const watch = process.argv.includes('--watch');

async function main() {
  const ctx = await esbuild.context({
    entryPoints: ['src/extension.ts'],
    bundle: true,
    format: 'cjs',
    minify: production,
    sourcemap: !production,
    sourcesContent: false,
    platform: 'node',
    outfile: 'out/extension.js',
    external: ['vscode'],
    logLevel: 'warning',
    plugins: [
      /* add to the end of plugins array */
      copyFilesPlugin,
      esbuildProblemMatcherPlugin
    ]
  });
  if (watch) {
    await ctx.watch();
  } else {
    await ctx.rebuild();
    await ctx.dispose();
  }
}

/**
 * @type {import('esbuild').Plugin}
 */
const copyFilesPlugin = {
  name: 'copy-files',
  setup(build) {
    build.onEnd(() => {
      const sourceFile = path.join(__dirname, '..', 'syntaxes', 'braise.tmLanguage.json');
      const targetDir = path.join(__dirname, 'out', 'syntaxes');
      const targetFile = path.join(targetDir, 'braise.tmLanguage.json');

      // Create target directory if it doesn't exist
      if (!fs.existsSync(targetDir)) {
        fs.mkdirSync(targetDir, { recursive: true });
      }

      // Copy the grammar file
      try {
        fs.copyFileSync(sourceFile, targetFile);
        console.log(`✓ Copied ${path.relative(__dirname, sourceFile)} to ${path.relative(__dirname, targetFile)}`);
      } catch (error) {
        console.error(`✘ Failed to copy grammar file: ${error.message}`);
      }
    });
  }
};

/**
 * @type {import('esbuild').Plugin}
 */
const esbuildProblemMatcherPlugin = {
  name: 'esbuild-problem-matcher',

  setup(build) {
    build.onStart(() => {
      console.log('[watch] build started');
    });
    build.onEnd(result => {
      result.errors.forEach(({ text, location }) => {
        console.error(`✘ [ERROR] ${text}`);
        if (location == null) return;
        console.error(`    ${location.file}:${location.line}:${location.column}:`);
      });
      console.log('[watch] build finished');
    });
  }
};

main().catch(e => {
  console.error(e);
  process.exit(1);
});
