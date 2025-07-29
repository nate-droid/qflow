import React, { useState } from 'react';
import CytoscapeGraph from '../components/CytoscapeGraph';

// Mock data to simulate a fetched workflow
const mockWorkflowData = {
    elements: [
        // Nodes
        { data: { id: 'task1', label: 'Generate Dataset', status: 'Succeeded', type: 'classical', params: { generator: 'make_moons', samples: 100 } } },
        { data: { id: 'task2', label: 'Quantum Kernel', status: 'Succeeded', type: 'quantum', circuit: 'OPENQASM 2.0;\\n...', params: { image: 'upcloud/quantum-svm:latest' } } },
        { data: { id: 'task3', label: 'Train SVM', status: 'Running', type: 'classical', params: { C: 1.0 } } },
        { data: { id: 'task4', label: 'Generate Plot', status: 'Pending', type: 'classical', params: { plot_name: 'qsvm-plot' } } },
        // Edges
        { data: { source: 'task1', target: 'task2' } },
        { data: { source: 'task2', target: 'task3' } },
        { data: { source: 'task3', target: 'task4' } }
    ],
    status: 'Running'
};

const DetailsPanel = ({ node, onShowResults }) => {
    if (!node) {
        return (
            <div className="text-center text-slate-400 pt-10">
                <svg xmlns="http://www.w3.org/2000/svg" className="mx-auto h-10 w-10 text-slate-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="1.5"><path strokeLinecap="round" strokeLinejoin="round" d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9 5.25h.008v.008H12v-.008z" /></svg>
                <p className="mt-2">Select a task node to view its details.</p>
            </div>
        );
    }

    return (
        <div className="space-y-5">
            <div className="bg-slate-900/50 p-4 rounded-lg border border-slate-700">
                <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">Status</h3>
                <p className="font-code text-sm font-semibold">{node.status}</p>
            </div>
            {node.circuit && (
                <div className="bg-slate-900/50 p-4 rounded-lg border border-slate-700">
                    <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">Circuit (OpenQASM)</h3>
                    <pre className="bg-slate-950 rounded-md p-3 text-sm font-code text-indigo-300 overflow-x-auto">
                        <code>{node.circuit}</code>
                    </pre>
                </div>
            )}
            <div className="bg-slate-900/50 p-4 rounded-lg border border-slate-700">
                <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">Parameters</h3>
                <pre className="bg-slate-950 rounded-md p-3 text-sm font-code text-amber-300 overflow-x-auto">
                    <code>{JSON.stringify(node.params, null, 2)}</code>
                </pre>
            </div>
            {node.status === 'Succeeded' && (
                <button onClick={() => onShowResults(node)} className="w-full btn btn-success">View Results</button>
            )}
        </div>
    );
};


export default function VisualizerPage() {
    const [workflow, setWorkflow] = useState(null);
    const [workflowNameInput, setWorkflowNameInput] = useState('my-quantum-workflow');
    const [namespaceInput, setNamespaceInput] = useState('default');
    const [selectedNode, setSelectedNode] = useState(null);
    const [isLoading, setIsLoading] = useState(false);

    const handleFetchWorkflow = async () => {
        setIsLoading(true);
        setSelectedNode(null);
        console.log(`Fetching workflow: ${workflowNameInput} in ${namespaceInput}`);

        // In a real app, you would make an API call here.
        // For now, we'll just use the mock data after a short delay.
        setTimeout(() => {
            // Add status class to each node for styling
            mockWorkflowData.elements.forEach(el => {
                if (el.data.id) { // It's a node
                    el.classes = `status-${el.data.status?.toLowerCase()}`;
                }
            });
            setWorkflow(mockWorkflowData);
            setIsLoading(false);
        }, 1000);
    };

    const handleNodeTap = (nodeData) => {
        setSelectedNode(nodeData);
    };

    const handleShowResults = (nodeData) => {
        // This is where you would open the results modal
        alert(`Showing results for task: ${nodeData.label}`);
    };

    // Render the "Fetch Workflow" overlay if no workflow is loaded yet
    if (!workflow) {
        return (
            <div className="absolute inset-0 bg-slate-900/80 flex items-center justify-center z-30">
                <div className="w-full max-w-md p-4 text-center">
                    <h2 className="mt-2 text-2xl font-bold text-white mb-2">Fetch a Workflow</h2>
                    <p className="text-slate-400 mb-6">Enter the name and namespace of a workflow to visualize.</p>
                    <div className="flex flex-col sm:flex-row gap-3 mb-4">
                        <input type="text" value={workflowNameInput} onChange={e => setWorkflowNameInput(e.target.value)} className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full" placeholder="Workflow Name" />
                        <input type="text" value={namespaceInput} onChange={e => setNamespaceInput(e.target.value)} className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full" placeholder="Namespace" />
                    </div>
                    <button onClick={handleFetchWorkflow} disabled={isLoading} className="btn btn-primary w-full">
                        {isLoading ? 'Fetching...' : 'Fetch & Visualize'}
                    </button>
                </div>
            </div>
        );
    }

    // Main view with graph and details panel
    return (
        <div className="flex-1 flex overflow-hidden h-full">
            <div className="flex-1 relative">
                <CytoscapeGraph
                    elements={workflow.elements}
                    onNodeTap={handleNodeTap}
                    style={{ width: '100%', height: '100%' }}
                />
            </div>
            <aside className="w-full md:w-1/3 lg:w-1/4 bg-slate-800/80 border-l border-slate-700/80 flex flex-col">
                <div className="p-6 overflow-y-auto flex-1">
                    <h2 className="text-xl font-bold text-white mb-6">Status: <span className="text-indigo-400 font-semibold">{workflow.status}</span></h2>
                    <DetailsPanel node={selectedNode} onShowResults={handleShowResults} />
                </div>
            </aside>
        </div>
    );
}