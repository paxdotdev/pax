export class ImageLoadPatch {
    public id?: number;
    public path?: string;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.path = jsonMessage["path"];
    }

    cleanUp(){
        this.id = undefined;
        this.path = '';
    }
}
