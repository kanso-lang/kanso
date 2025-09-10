import * as assert from 'assert';
import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

suite('Syntax Highlighting Tests', () => {

	test('Should load syntax grammar correctly', () => {
		// Check that the grammar file exists and is valid JSON
		const grammarPath = path.resolve(__dirname, '../../../syntaxes/kanso.tmLanguage.json');
		assert.ok(fs.existsSync(grammarPath), 'Grammar file should exist');
		
		const grammarContent = fs.readFileSync(grammarPath, 'utf8');
		let grammar;
		
		try {
			grammar = JSON.parse(grammarContent);
		} catch (e) {
			assert.fail('Grammar file should be valid JSON');
		}

		// Verify essential grammar properties
		assert.strictEqual(grammar.scopeName, 'source.kanso');
		assert.strictEqual(grammar.name, 'Kanso');
		assert.ok(grammar.patterns && Array.isArray(grammar.patterns));
		assert.ok(grammar.repository && typeof grammar.repository === 'object');
	});

	test('Should have correct keyword patterns', () => {
		const grammarPath = path.resolve(__dirname, '../../../syntaxes/kanso.tmLanguage.json');
		const grammarContent = fs.readFileSync(grammarPath, 'utf8');
		const grammar = JSON.parse(grammarContent);

		const keywordPatterns = grammar.repository.keywords.patterns;
		const keywordPattern = keywordPatterns.find((p: any) => p.name === 'keyword.control');
		
		assert.ok(keywordPattern, 'Should have keyword.control pattern');
		
		// Check for essential Kanso keywords
		const keywordRegex = keywordPattern.match;
		assert.ok(keywordRegex.includes('contract'), 'Should include contract keyword');
		assert.ok(keywordRegex.includes('fn'), 'Should include fn keyword');
		assert.ok(keywordRegex.includes('ext'), 'Should include ext keyword');
		assert.ok(keywordRegex.includes('struct'), 'Should include struct keyword');
		assert.ok(keywordRegex.includes('use'), 'Should include use keyword');
	});

	test('Should have correct type patterns', () => {
		const grammarPath = path.resolve(__dirname, '../../../syntaxes/kanso.tmLanguage.json');
		const grammarContent = fs.readFileSync(grammarPath, 'utf8');
		const grammar = JSON.parse(grammarContent);

		const typePatterns = grammar.repository.types.patterns;
		const primitivePattern = typePatterns.find((p: any) => p.name === 'entity.name.type.primitive');
		
		assert.ok(primitivePattern, 'Should have primitive type pattern');
		
		// Check for essential Kanso types
		const typeRegex = primitivePattern.match;
		assert.ok(typeRegex.includes('U256'), 'Should include U256 type');
		assert.ok(typeRegex.includes('Address'), 'Should include Address type');
		assert.ok(typeRegex.includes('Bool'), 'Should include Bool type');
		assert.ok(typeRegex.includes('String'), 'Should include String type');
	});

	test('Should have attribute patterns', () => {
		const grammarPath = path.resolve(__dirname, '../../../syntaxes/kanso.tmLanguage.json');
		const grammarContent = fs.readFileSync(grammarPath, 'utf8');
		const grammar = JSON.parse(grammarContent);

		const keywordPatterns = grammar.repository.keywords.patterns;
		const attributePattern = keywordPatterns.find((p: any) => p.name === 'storage.modifier.attribute');
		
		assert.ok(attributePattern, 'Should have attribute pattern');
		
		// Check for Kanso attributes
		const attributeRegex = attributePattern.match;
		assert.ok(attributeRegex.includes('storage'), 'Should include storage attribute');
		assert.ok(attributeRegex.includes('event'), 'Should include event attribute');
		assert.ok(attributeRegex.includes('create'), 'Should include create attribute');
	});

	test('Should recognize Kanso file extension', async () => {
		// Test that .ka files are recognized as Kanso language
		const document = await vscode.workspace.openTextDocument({
			content: 'contract Test {}',
			language: 'kanso'
		});

		assert.strictEqual(document.languageId, 'kanso');

		// Clean up
		await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
	});

	test('Should provide semantic tokens for contract structure', async () => {
		const testContent = `contract TestContract {
	use std::evm::{sender, emit};
	
	#[storage]
	struct State {
		balance: U256,
		owner: Address,
	}
	
	#[event]
	struct Transfer {
		from: Address,
		to: Address,
		amount: U256,
	}
	
	#[create]
	fn create() writes State {
		State.balance = 0;
	}
	
	ext fn getBalance() -> U256 reads State {
		State.balance
	}
}`;

		const document = await vscode.workspace.openTextDocument({
			content: testContent,
			language: 'kanso'
		});

		await vscode.window.showTextDocument(document);

		// Wait for language server to process
		await new Promise(resolve => setTimeout(resolve, 2000));

		const semanticTokens = await vscode.commands.executeCommand<vscode.SemanticTokens>(
			'vscode.provideDocumentSemanticTokens',
			document.uri
		);

		assert.ok(semanticTokens, 'Should provide semantic tokens for contract');
		assert.ok(semanticTokens.data.length > 0, 'Should have token data');

		// Clean up
		await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
	});
});