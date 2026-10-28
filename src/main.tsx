import React, { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { createRoot } from 'react-dom/client';
import App from './App';
import './style.css';
import './theme.css';

class StartupBoundary extends React.Component<React.PropsWithChildren, {error:string|null}> {
  state:{error:string|null}={error:null};
  static getDerivedStateFromError(error:unknown){return {error:String(error)};}
  componentDidCatch(error:Error){void invoke('startup_diagnostic',{message:'React render failed: '+error.message}).catch(()=>{});}
  render(){return this.state.error ? <div role="alert" style={{padding:32,color:'white',background:'#102943'}}>Proton Medic could not render: {this.state.error}</div> : this.props.children;}
}
function Startup(){
  useEffect(()=>{const banner=document.getElementById('startup-status');if(banner)banner.hidden=true;
    void invoke('startup_diagnostic',{message:'React committed application UI'}).catch(()=>{});
  },[]);
  return <App/>;
}
const root = document.getElementById('root');
if (!root) throw new Error('Missing root element');
createRoot(root).render(<React.StrictMode><StartupBoundary><Startup /></StartupBoundary></React.StrictMode>);
