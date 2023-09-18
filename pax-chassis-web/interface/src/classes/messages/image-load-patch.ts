export class ImageLoadPatch {
    public id_chain?: number[];
    public path?: string;

    fromPatch(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.path = jsonMessage["path"];
    }

    cleanUp(){
        this.id_chain = [];
        this.path = '';
    }
}
