import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("visualizer", "routes/visualizer.tsx"),
  route("svm", "routes/svm.tsx"),
  route("qcbm", "routes/qcbm.tsx"),
  route("qasm", "routes/qasm.tsx"),
  route("ml-svm", "routes/ml-svm.tsx"),
  route("simulator", "routes/simulator.tsx"),
] satisfies RouteConfig;
