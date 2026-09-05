#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "WorldVMSubsystem.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnWorldVMModuleLoaded, const FString&, ModuleName);

/**
 * Unreal Engine GameInstance Subsystem managing the WorldVM runtime.
 */
UCLASS()
class WORLDVM_API UWorldVMSubsystem : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    virtual void Initialize(FSubsystemCollectionBase& Collection) override;
    virtual void Deinitialize() override;

    UFUNCTION(BlueprintCallable, Category = "WorldVM")
    bool LoadWorldMod(const FString& PackageFilePath);

    UFUNCTION(BlueprintCallable, Category = "WorldVM")
    bool EmitWorldVMEvent(const FString& ModuleId, const FString& EventName, const FString& JsonPayload);

    UPROPERTY(BlueprintAssignable, Category = "WorldVM")
    FOnWorldVMModuleLoaded OnModuleLoaded;

private:
    void* WorldVMRuntimeHandle = nullptr;
};
