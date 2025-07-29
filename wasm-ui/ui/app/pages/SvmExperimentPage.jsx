import React, { useState } from 'react';

// A simple status indicator component
const StatusDisplay = ({ statusText }) => (
    <div className="text-slate-400 flex items-center gap-3">
        <svg className="animate-spin h-5 w-5 text-indigo-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
        <span className="font-medium">{statusText}</span>
    </div>
);

export default function SvmExperimentPage() {
    const [formState, setFormState] = useState({
        dsGenerator: 'make_moons',
        dsSamples: 100,
        dsNoise: 0.1,
        dsTestSize: 0.3,
        kernelImage: 'upcloud/quantum-svm:latest',
        trainerC: 1.0,
        outputModelName: 'qsvm-model',
        outputPlotName: 'qsvm-decision-boundary',
    });
    const [statusText, setStatusText] = useState('');
    const [error, setError] = useState('');
    const [results, setResults] = useState(null); // Will hold { plotUrl, metricsText }

    const handleInputChange = (e) => {
        const { id, value, type } = e.target;
        setFormState(prevState => ({
            ...prevState,
            [id]: type === 'number' ? parseFloat(value) : value,
        }));
    };

    const handleRunExperiment = async () => {
        setStatusText('Submitting workflow...');
        setError('');
        setResults(null);

        const payload = {
            metadata: { name: `qsvm-${formState.outputModelName}-${Date.now()}` },
            spec: {
                tasks: [{
                    name: "quantum-svm-pipeline",
                    spec: {
                        qsvm: {
                            dataset: {
                                generator: formState.dsGenerator,
                                samples: formState.dsSamples,
                                noise: formState.dsNoise,
                                test_size: formState.dsTestSize
                            },
                            kernel: { image: formState.kernelImage },
                            trainer: { c: formState.trainerC },
                            output: {
                                model_name: formState.outputModelName,
                                plot_name: formState.outputPlotName
                            }
                        }
                    }
                }]
            }
        };

        console.log("Submitting Payload:", payload);
        // In a real app, you'd have API calls and polling here.
        // We will simulate the process with timeouts.
        setTimeout(() => {
            setStatusText('Workflow running, polling for status...');
            setTimeout(() => {
                // Simulate success and fetching results
                setStatusText('');
                // Use a placeholder image for the plot
                const mockPlotUrl = 'https://via.placeholder.com/600x400.png?text=Decision+Boundary+Plot';
                const mockMetrics = { accuracy: 0.95, precision: 0.94, recall: 0.96 };
                setResults({
                    plotUrl: mockPlotUrl,
                    metricsText: JSON.stringify(mockMetrics, null, 2)
                });
            }, 4000);
        }, 2000);
    };

    return (
        <div className="p-8">
            <div className="max-w-4xl mx-auto">
                <h2 className="text-2xl font-bold text-white mb-2">Run a New Quantum SVM Experiment</h2>
                <p className="text-slate-400 mb-6">Specify the parameters for the QuantumSVMWorkflow and click "Run Experiment" to submit.</p>

                <div className="space-y-8">
                    {/* Dataset Section */}
                    <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
                        <h3 className="text-lg font-semibold text-indigo-400 mb-4">Dataset Configuration</h3>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div>
                                <label htmlFor="dsGenerator" className="block text-sm">Generator</label>
                                <input type="text" id="dsGenerator" value={formState.dsGenerator} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                            </div>
                            {/* Add other inputs similarly... */}
                            <div>
                                <label htmlFor="dsSamples" className="block text-sm">Samples</label>
                                <input type="number" id="dsSamples" value={formState.dsSamples} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                            </div>
                            <div>
                                <label htmlFor="dsNoise" className="block text-sm">Noise</label>
                                <input type="number" id="dsNoise" step="0.05" value={formState.dsNoise} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                            </div>
                            <div>
                                <label htmlFor="dsTestSize" className="block text-sm">Test Size</label>
                                <input type="number" id="dsTestSize" step="0.05" value={formState.dsTestSize} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                            </div>
                        </div>
                    </div>

                    {/* Kernel & Trainer Section */}
                    <div className="grid md:grid-cols-2 gap-8">
                        <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
                            <h3 className="text-lg font-semibold text-indigo-400 mb-4">Kernel</h3>
                            <label htmlFor="kernelImage" className="block text-sm">Container Image</label>
                            <input type="text" id="kernelImage" value={formState.kernelImage} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                        </div>
                        <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
                            <h3 className="text-lg font-semibold text-indigo-400 mb-4">Trainer</h3>
                            <label htmlFor="trainerC" className="block text-sm">C (Regularization)</label>
                            <input type="number" id="trainerC" value={formState.trainerC} onChange={handleInputChange} className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1" />
                        </div>
                    </div>

                    {/* Submission Section */}
                    <div className="pt-4 flex justify-end items-center gap-6">
                        {statusText && <StatusDisplay statusText={statusText} />}
                        <button onClick={handleRunExperiment} disabled={!!statusText} className="btn btn-primary">Run Experiment</button>
                    </div>
                </div>

                {/* Error Display */}
                {error && (
                    <div className="mt-6 p-4 rounded-md bg-red-500/20 text-red-300 border border-red-500/30">
                        <h4 className="font-bold mb-2">An Error Occurred</h4>
                        <p className="font-code text-sm">{error}</p>
                    </div>
                )}

                {/* Results Area */}
                {results && (
                    <div className="mt-12">
                        <h2 className="text-2xl font-bold text-white mb-6">Results</h2>
                        <div className="space-y-8">
                            <div>
                                <h3 className="text-lg font-semibold text-indigo-400 mb-4">Decision Boundary Plot</h3>
                                <div className="p-4 bg-slate-800 rounded-lg border border-slate-700">
                                    <img src={results.plotUrl} alt="Decision Boundary Plot" className="max-w-full h-auto rounded" />
                                </div>
                            </div>
                            <div>
                                <h3 className="text-lg font-semibold text-indigo-400 mb-4">Performance Metrics</h3>
                                <div className="p-4 bg-slate-950 rounded-lg border border-slate-700">
                                    <pre className="font-code text-sm text-slate-300">{results.metricsText}</pre>
                                </div>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}