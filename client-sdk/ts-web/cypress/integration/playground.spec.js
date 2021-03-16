/// <reference types="cypress" />

describe('playground', () => {
    it('finishes', () => {
        cy.visit('http://localhost:42280/');
        cy.contains('lappinite', {timeout: 60_000});
    });
});
