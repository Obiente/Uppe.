export async function boundedBody(request: Request, limit: number): Promise<Uint8Array> {
  const reader=request.body?.getReader();
  if(!reader) return new Uint8Array();
  const chunks:Uint8Array[]=[]; let size=0;
  try {
    while(true) { const {done,value}=await reader.read(); if(done) break; size+=value.byteLength;
      if(size>limit) { await reader.cancel(); throw new Error('Request body too large'); } chunks.push(value);
    }
  } finally { reader.releaseLock(); }
  const data=new Uint8Array(size);let offset=0;for(const chunk of chunks){data.set(chunk,offset);offset+=chunk.byteLength;}return data;
}
