import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Kanso Extension Test Suite', () => {
	vscode.window.showInformationMessage('Start all tests.');

	test('Extension should be present', () => {
		assert.ok(vscode.extensions.getExtension('kanso-lang.kanso'));
	});

	test('Should activate extension', async () => {
		const extension = vscode.extensions.getExtension('kanso-lang.kanso');
		if (extension) {
			await extension.activate();
			assert.strictEqual(extension.isActive, true);
		}
	});

	test('Should register Kanso language', () => {
		const languages = vscode.languages.getLanguages();
		
		// Check if .ka files are associated with kanso language
		return languages.then(langs => {
			assert.ok(langs.includes('kanso'), 'Kanso language should be registered');
		});
	});

	test('Should provide language server features', async () => {
		// Create a temporary Kanso file
		const testContent = `contract TestContract {
	use std::evm::{sender, emit};
	
	#[storage]
	struct State {
		balance: U256,
	}
	
	ext fn getBalance() -> U256 reads State {
		State.balance
	}
}`;

		const document = await vscode.workspace.openTextDocument({
			content: testContent,
			language: 'kanso'
		});

		const editor = await vscode.window.showTextDocument(document);

		// Wait a bit for the language server to process
		await new Promise(resolve => setTimeout(resolve, 2000));

		// Test semantic tokens (syntax highlighting)
		const semanticTokens = await vscode.commands.executeCommand<vscode.SemanticTokens>(
			'vscode.provideDocumentSemanticTokens',
			document.uri
		);

		assert.ok(semanticTokens, 'Should provide semantic tokens');
		assert.ok(semanticTokens.data.length > 0, 'Semantic tokens should not be empty');

		// Test completion provider
		const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
			'vscode.executeCompletionItemProvider',
			document.uri,
			new vscode.Position(8, 10) // After "State."
		);

		assert.ok(completions, 'Should provide completions');

		// Clean up
		await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
	});

	test('Should provide diagnostics for syntax errors', async () => {
		// Create a Kanso file with syntax errors
		const testContentWithErrors = `contract BadContract {
	ext fn missingType() {
		let x = ;
		missing_semicolon
	}
}`;

		const document = await vscode.workspace.openTextDocument({
			content: testContentWithErrors,
			language: 'kanso'
		});

		await vscode.window.showTextDocument(document);

		// Wait for diagnostics to be generated
		await new Promise(resolve => setTimeout(resolve, 3000));

		const diagnostics = vscode.languages.getDiagnostics(document.uri);
		assert.ok(diagnostics.length > 0, 'Should provide diagnostics for syntax errors');

		// Check that we have error-level diagnostics
		const errors = diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
		assert.ok(errors.length > 0, 'Should have error-level diagnostics');

		// Clean up
		await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
	});

	test('Should handle struct field completion', async () => {
		const testContent = `contract TestContract {
	#[storage]
	struct State {
		balance: U256,
		owner: Address,
		name: String,
	}
	
	ext fn test() -> U256 reads State {
		State.
	}
}`;

		const document = await vscode.workspace.openTextDocument({
			content: testContent,
			language: 'kanso'
		});

		await vscode.window.showTextDocument(document);

		// Wait for language server to process
		await new Promise(resolve => setTimeout(resolve, 2000));

		// Test completion after "State."
		const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
			'vscode.executeCompletionItemProvider',
			document.uri,
			new vscode.Position(9, 8) // After "State."
		);

		assert.ok(completions, 'Should provide completions after struct access');
		
		if (completions.items) {
			const fieldNames = completions.items.map(item => item.label);
			assert.ok(fieldNames.some(name => 
				typeof name === 'string' ? name.includes('balance') : false
			), 'Should suggest struct fields');
		}

		// Clean up
		await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
	});
});