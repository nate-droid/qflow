import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';

const Notification = ({ message, type }) => {
    if (!message) return null;
    const style = type === 'success' ? 'bg-green-500/20 text-green-300' : 'bg-red-500/20 text-red-300';
    return <div className={`p-4 mb-4 rounded-md ${style}`}>{message}</div>;
};

export default function QcbmSubmitterPage() {
    const [formState, setFormState] = useState({
        workflowName: 'my-qcbm-training',
        namespace: 'default',
        taskName: 'qcbm-training-task',
        image: 'qcbm-runner:latest',
        ansatz: 'MyAnsatz',
        trainingData: '/path/to/data1.csv\n/path/to/data2.csv',
        optName: 'Adam',
        optEpochs: 100,
        optLr: 0.01,
        optParams: '[0.1, 0.2, 0.3, 0.4]'
    });
    const [notification, setNotification] = useState({ message: '', type: '' });
    const [isSubmitting, setIsSubmitting] = useState(false);
    const navigate = useNavigate();

    const handleInputChange = (e) => {
        const { id, value } = e.target;
        setFormState(prevState => ({ ...prevState, [id]: value }));
    };

    const handleSubmit = async () => {
        setIsSubmitting(true);
        setNotification({ message: '', type: '' });

        console.log("Submitting QCBM Workflow:", formState);

        // Simulate API call
        setTimeout(() => {
            setIsSubmitting(false);
            setNotification({ message: `Workflow "${formState.workflowName}" created! Redirecting...`, type: 'success' });
            setTimeout(() => navigate('/visualizer'), 2000);
        }, 1500);
    };

    return (
        <div className="p-8">
            <div className="max-w-4xl mx-auto">
                <h2 className="text-2xl font-bold text-white mb-2">Submit QCBM Workflow</h2>
                <p className="text-slate-400 mb-6">Define a QCBM task and its parameters to create a new workflow.</p>

                <Notification message={notification.message} type={notification.type} />

                <div className="space-y-6">
                    {/* Form sections go here */}
                    <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
                        <h3 className="text-lg font-semibold text-indigo-400 mb-4">Workflow Details</h3>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div>
                                <label htmlFor="workflowName" className="block text-sm">Workflow Name</label>
                                <input type="text" id="workflowName" value={formState.workflowName} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                            </div>
                            <div>
                                <label htmlFor="namespace" className="block text-sm">Namespace</label>
                                <input type="text" id="namespace" value={formState.namespace} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                            </div>
                        </div>
                    </div>
                    <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
                        <h3 className="text-lg font-semibold text-indigo-400 mb-4">Task Specification</h3>
                        <div className="space-y-4">
                            <label htmlFor="taskName" className="block text-sm">Task Name</label>
                            <input type="text" id="taskName" value={formState.taskName} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3" />
                            <label htmlFor="trainingData" className="block text-sm">Training Data (one per line)</label>
                            <textarea id="trainingData" value={formState.trainingData} onChange={handleInputChange} rows="3" className="w-full bg-slate-950 font-code p-3 rounded-md"></textarea>
                        </div>
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