import React, { useState } from 'react';

// You can create a reusable Notification component
const Notification = ({ message, type }) => {
    if (!message) return null;
    const baseClasses = 'p-4 mb-4 rounded-md';
    const typeClasses = type === 'success'
        ? 'bg-green-500/20 text-green-300'
        : 'bg-red-500/20 text-red-300';
    return <div className={`${baseClasses} ${typeClasses}`}>{message}</div>;
};

export default function QasmSubmitterPage() {
    const [workflowName, setWorkflowName] = useState('my-bell-state-exp');
    const [namespace, setNamespace] = useState('default');
    const [qasmCode, setQasmCode] = useState('OPENQASM 2.0;\\ninclude "qelib1.inc";\\n\\nqreg q[2];\\ncreg c[2];\\n\\nh q[0];\\ncx q[0], q[1];\\nmeasure q -> c;');
    const [notification, setNotification] = useState({ message: '', type: '' });
    const [isSubmitting, setIsSubmitting] = useState(false);

    const handleSubmit = async () => {
        setIsSubmitting(true);
        setNotification({ message: '', type: '' });

        const payload = { /* ... construct payload ... */ };

        try {
            // const response = await fetch(`${API_BASE_URL}/api/workflows/...`, { ... });
            // if (!response.ok) throw new Error('Failed to submit.');

            // Mock success for now
            console.log("Submitting:", { workflowName, namespace, qasmCode });
            setNotification({ message: 'Workflow submitted successfully!', type: 'success' });
        } catch (error) {
            setNotification({ message: error.message, type: 'error' });
        } finally {
            setIsSubmitting(false);
        }
    };

    return (
        <div className="p-8">
            <div className="max-w-4xl mx-auto">
                <h2 className="text-2xl font-bold text-white mb-2">Submit OpenQASM Workflow</h2>
                <p className="text-slate-400 mb-6">Paste your OpenQASM 2.0 code below to create and run a new workflow.</p>

                <Notification message={notification.message} type={notification.type} />

                <div className="space-y-4">
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                            <label htmlFor="qasm-workflow-name" className="block text-sm font-medium text-slate-300 mb-1">Workflow Name</label>
                            <input type="text" id="qasm-workflow-name" value={workflowName} onChange={e => setWorkflowName(e.target.value)} className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full" />
                        </div>
                        <div>
                            <label htmlFor="qasm-namespace" className="block text-sm font-medium text-slate-300 mb-1">Namespace</label>
                            <input type="text" id="qasm-namespace" value={namespace} onChange={e => setNamespace(e.target.value)} className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full" />
                        </div>
                    </div>
                    <div>
                        <label htmlFor="qasm-input" className="block text-sm font-medium text-slate-300 mb-1">OpenQASM 2.0 Code</label>
                        <textarea id="qasm-input" rows="15" value={qasmCode} onChange={e => setQasmCode(e.target.value)} className="w-full bg-slate-950 border border-slate-600 rounded-md p-3 font-code text-indigo-300"></textarea>
                    </div>
                    <div className="flex justify-end">
                        <button onClick={handleSubmit} disabled={isSubmitting} className="btn btn-primary">
                            {isSubmitting ? 'Submitting...' : 'Submit Workflow'}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}