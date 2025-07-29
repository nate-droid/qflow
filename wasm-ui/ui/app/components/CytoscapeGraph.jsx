import React, { useRef, useEffect } from 'react';
import cytoscape from 'cytoscape';
import dagre from 'cytoscape-dagre';

cytoscape.use(dagre);

const CytoscapeGraph = ({ elements, style, onNodeTap }) => {
    const cyRef = useRef(null);

    useEffect(() => {
        const cy = cytoscape({
            container: cyRef.current,
            elements: elements,
            layout: { name: 'dagre' },
            style: [
                { selector: 'node', style: { 'background-color': '#1e293b', 'border-color': '#475569', 'border-width': 2, 'label': 'data(label)', 'color': '#e2e8f0', 'font-size': '12px', 'text-valign': 'center', 'text-halign': 'center', 'width': '140px', 'height': '50px', 'shape': 'round-rectangle', 'transition-property': 'background-color, border-color', 'transition-duration': '0.3s' } },
                { selector: 'edge', style: { 'width': 2, 'line-color': '#64748b', 'target-arrow-color': '#64748b', 'target-arrow-shape': 'triangle', 'curve-style': 'bezier' } },
                { selector: 'node:selected', style: { 'border-color': '#818cf8', 'background-color': '#312e81' } },
                { selector: '.status-succeeded', style: { 'border-color': '#22c55e' } },
                { selector: '.status-running', style: { 'border-color': '#f59e0b', 'line-style': 'dashed', 'border-dash-pattern': [6, 3] } },
                { selector: '.status-failed', style: { 'border-color': '#ef4444' } },
                { selector: '.status-pending', style: { 'border-color': '#64748b' } },
            ]
        });

        cy.on('tap', 'node', (evt) => {
            if (onNodeTap) onNodeTap(evt.target);
        });

        // Cleanup function to destroy the graph instance
        return () => {
            cy.destroy();
        };
    }, [elements, onNodeTap]); // Re-run effect if elements change

    return <div ref={cyRef} style={style} />;
};

export default CytoscapeGraph;